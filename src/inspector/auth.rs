use crate::{DebuggerError, Result};
use serde::{Deserialize, Serialize};
use soroban_sdk::{
    testutils::{AuthorizedFunction, AuthorizedInvocation},
    Env,
};

/// Status of an authorization node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthStatus {
    /// Authorization was successfully recorded.
    Authorized,
    /// Authorization was required but not provided (missing).
    Missing,
    /// Authorization check failed at runtime.
    Failed,
}

impl AuthStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthStatus::Authorized => "authorized",
            AuthStatus::Missing => "missing",
            AuthStatus::Failed => "failed",
        }
    }
}

/// Represents a node in the authorization tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthNode {
    /// The address that authorized this invocation (empty string when unknown).
    pub address: String,
    /// The contract function being authorized.
    pub function: String,
    /// The contract being called.
    pub contract_id: String,
    /// Whether this authorization was successful.
    pub status: AuthStatus,
    /// Child invocations authorized under this node.
    pub sub_invocations: Vec<AuthNode>,
}

impl AuthNode {
    /// Returns true if this node or any descendant has a non-Authorized status.
    pub fn has_failures(&self) -> bool {
        if self.status != AuthStatus::Authorized {
            return true;
        }
        self.sub_invocations.iter().any(|s| s.has_failures())
    }
}

pub struct AuthInspector;

impl AuthInspector {
    /// Extract the authorization tree from the environment, capturing addresses.
    pub fn get_auth_tree(env: &Env) -> Result<Vec<AuthNode>> {
        let recorded_auths = env.auths();
        let mut nodes = Vec::new();

        for (address, invocation) in recorded_auths {
            let address_str = format!("{:?}", address);
            nodes.push(Self::convert_invocation(&invocation, &address_str));
        }

        Ok(nodes)
    }

    fn convert_invocation(inv: &AuthorizedInvocation, address: &str) -> AuthNode {
        let (function, contract_id) = match &inv.function {
            AuthorizedFunction::Contract(call) => {
                let contract_id = format!("{:?}", call.0);
                let function = format!("{:?}({:?})", call.1, call.2);
                (function, contract_id)
            }
            AuthorizedFunction::CreateContractHostFn(create_fn) => {
                let contract_id = "Host".to_string();
                let function = format!("create_contract({:?})", create_fn);
                (function, contract_id)
            }
            AuthorizedFunction::CreateContractV2HostFn(create_fn) => {
                let contract_id = "Host".to_string();
                let function = format!("create_contract_v2({:?})", create_fn);
                (function, contract_id)
            }
        };

        // Sub-invocations share the same authorizing address.
        let sub_invocations = inv
            .sub_invocations
            .iter()
            .map(|s| Self::convert_invocation(s, address))
            .collect();

        AuthNode {
            address: address.to_string(),
            function,
            contract_id,
            status: AuthStatus::Authorized,
            sub_invocations,
        }
    }

    /// Build a set of failed/missing auth nodes from a list of required invocations
    /// that were NOT present in the recorded auth tree.
    pub fn build_failed_nodes(required: &[(&str, &str, &str)]) -> Vec<AuthNode> {
        required
            .iter()
            .map(|(address, contract_id, function)| AuthNode {
                address: address.to_string(),
                function: function.to_string(),
                contract_id: contract_id.to_string(),
                status: AuthStatus::Missing,
                sub_invocations: vec![],
            })
            .collect()
    }

    /// Display the authorization tree to stdout with ANSI color coding.
    ///
    /// - Green: authorized
    /// - Red: failed or missing
    pub fn display(nodes: &[AuthNode]) {
        if nodes.is_empty() {
            println!("  (No authorizations recorded)");
            return;
        }

        for node in nodes {
            Self::print_node(node, 0, true);
        }
    }

    /// Display with a summary line showing overall pass/fail.
    pub fn display_with_summary(nodes: &[AuthNode]) {
        if nodes.is_empty() {
            println!("  (No authorizations recorded)");
            return;
        }

        let total = Self::count_nodes(nodes);
        let failed = Self::count_failed(nodes);

        for node in nodes {
            Self::print_node(node, 0, true);
        }

        println!();
        if failed == 0 {
            println!(
                "  {}",
                Self::green(&format!("[PASS] All {} authorization(s) succeeded", total))
            );
        } else {
            println!(
                "  {}",
                Self::red(&format!(
                    "[FAIL] {}/{} authorization(s) failed or missing",
                    failed, total
                ))
            );
        }
    }

    fn print_node(node: &AuthNode, depth: usize, is_last: bool) {
        let indent = "    ".repeat(depth);
        let branch = if depth == 0 {
            "".to_string()
        } else if is_last {
            format!("{}└── ", "    ".repeat(depth.saturating_sub(1)))
        } else {
            format!("{}├── ", "    ".repeat(depth.saturating_sub(1)))
        };

        let status_label = match node.status {
            AuthStatus::Authorized => Self::green("[OK]"),
            AuthStatus::Missing => Self::red("[MISSING]"),
            AuthStatus::Failed => Self::red("[FAILED]"),
        };

        // Address line (only at root level or when it differs from parent)
        if depth == 0 && !node.address.is_empty() {
            println!("{}Signer: {}", indent, Self::dim(&node.address));
        }

        let line = format!(
            "{}{} {} [Contract: {}]",
            branch, status_label, node.function, node.contract_id
        );

        println!("{}", line);

        let child_count = node.sub_invocations.len();
        for (i, sub) in node.sub_invocations.iter().enumerate() {
            Self::print_node(sub, depth + 1, i == child_count - 1);
        }
    }

    fn count_nodes(nodes: &[AuthNode]) -> usize {
        nodes
            .iter()
            .map(|n| 1 + Self::count_nodes(&n.sub_invocations))
            .sum()
    }

    fn count_failed(nodes: &[AuthNode]) -> usize {
        nodes
            .iter()
            .map(|n| {
                let self_failed = if n.status != AuthStatus::Authorized {
                    1
                } else {
                    0
                };
                self_failed + Self::count_failed(&n.sub_invocations)
            })
            .sum()
    }

    // ── ANSI helpers ──────────────────────────────────────────────────────────

    fn green(s: &str) -> String {
        if Self::colors_enabled() {
            format!("\x1b[32m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }

    fn red(s: &str) -> String {
        if Self::colors_enabled() {
            format!("\x1b[31m{}\x1b[0m", s)
        } else {
            format!("[!] {}", s)
        }
    }

    fn dim(s: &str) -> String {
        if Self::colors_enabled() {
            format!("\x1b[2m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }

    fn colors_enabled() -> bool {
        // Respect the same NO_COLOR convention used by the rest of the tool.
        std::env::var_os("NO_COLOR").is_none()
    }

    /// Return the authorization tree as a pretty-printed JSON string.
    pub fn to_json(nodes: &[AuthNode]) -> Result<String> {
        serde_json::to_string_pretty(nodes).map_err(|e| {
            DebuggerError::FileError(format!("Failed to serialize auth nodes: {}", e)).into()
        })
    }

    /// Return the authorization tree as a `serde_json::Value` for embedding
    /// into a larger JSON document.
    pub fn to_json_value(nodes: &[AuthNode]) -> serde_json::Value {
        serde_json::to_value(nodes).unwrap_or(serde_json::Value::Null)
    }
}
