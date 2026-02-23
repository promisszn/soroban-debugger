use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// WASM value type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WasmType {
    I32,
    I64,
    F32,
    F64,
    V128,
    FuncRef,
    ExternRef,
    Unknown,
}

impl fmt::Display for WasmType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WasmType::I32 => write!(f, "i32"),
            WasmType::I64 => write!(f, "i64"),
            WasmType::F32 => write!(f, "f32"),
            WasmType::F64 => write!(f, "f64"),
            WasmType::V128 => write!(f, "v128"),
            WasmType::FuncRef => write!(f, "funcref"),
            WasmType::ExternRef => write!(f, "externref"),
            WasmType::Unknown => write!(f, "?"),
        }
    }
}

/// A function signature extracted from a WASM module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSignature {
    pub name: String,
    pub params: Vec<WasmType>,
    pub results: Vec<WasmType>,
}

impl fmt::Display for FunctionSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let params: Vec<String> = self.params.iter().map(|t| t.to_string()).collect();
        let results: Vec<String> = self.results.iter().map(|t| t.to_string()).collect();
        write!(f, "{}({}) -> [{}]", self.name, params.join(", "), results.join(", "))
    }
}

/// A breaking change detected between two contract versions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BreakingChange {
    FunctionRemoved {
        name: String,
    },
    ParameterCountChanged {
        name: String,
        old_count: usize,
        new_count: usize,
    },
    ParameterTypeChanged {
        name: String,
        index: usize,
        old_type: WasmType,
        new_type: WasmType,
    },
    ReturnTypeChanged {
        name: String,
        old_types: Vec<WasmType>,
        new_types: Vec<WasmType>,
    },
}

impl fmt::Display for BreakingChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BreakingChange::FunctionRemoved { name } => {
                write!(f, "[REMOVED] {}", name)
            }
            BreakingChange::ParameterCountChanged { name, old_count, new_count } => {
                write!(f, "[PARAMS_CHANGED] {}: {} params -> {} params", name, old_count, new_count)
            }
            BreakingChange::ParameterTypeChanged { name, index, old_type, new_type } => {
                write!(f, "[PARAM_TYPE] {} param[{}]: {} -> {}", name, index, old_type, new_type)
            }
            BreakingChange::ReturnTypeChanged { name, old_types, new_types } => {
                let old: Vec<String> = old_types.iter().map(|t| t.to_string()).collect();
                let new: Vec<String> = new_types.iter().map(|t| t.to_string()).collect();
                write!(f, "[RETURN_TYPE] {}: [{}] -> [{}]", name, old.join(", "), new.join(", "))
            }
        }
    }
}

/// A non-breaking change detected between two contract versions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NonBreakingChange {
    FunctionAdded { name: String },
}

impl fmt::Display for NonBreakingChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NonBreakingChange::FunctionAdded { name } => write!(f, "[ADDED] {}", name),
        }
    }
}

/// Execution result comparison when --test-inputs is provided
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionDiff {
    pub function: String,
    pub args: String,
    pub old_result: String,
    pub new_result: String,
    pub outputs_match: bool,
}

/// The full compatibility report
#[derive(Debug, Serialize, Deserialize)]
pub struct CompatibilityReport {
    pub is_compatible: bool,
    pub old_wasm_path: String,
    pub new_wasm_path: String,
    pub breaking_changes: Vec<BreakingChange>,
    pub non_breaking_changes: Vec<NonBreakingChange>,
    pub old_functions: Vec<crate::utils::wasm::ContractFunctionSignature>,
    pub new_functions: Vec<crate::utils::wasm::ContractFunctionSignature>,
    pub execution_diffs: Vec<ExecutionDiff>,
}

pub struct UpgradeAnalyzer;

impl UpgradeAnalyzer {
    /// Analyze two WASM binaries and produce a compatibility report
    pub fn analyze(
        old_wasm: &[u8],
        new_wasm: &[u8],
        old_path: &str,
        new_path: &str,
        execution_diffs: Vec<ExecutionDiff>,
    ) -> Result<CompatibilityReport> {
        let old_functions = crate::utils::wasm::parse_function_signatures(old_wasm)?;
        let new_functions = crate::utils::wasm::parse_function_signatures(new_wasm)?;

        let (breaking_changes, non_breaking_changes) =
            Self::diff_signatures(&old_functions, &new_functions);

        let has_execution_mismatches = execution_diffs.iter().any(|d| !d.outputs_match);
        let is_compatible = breaking_changes.is_empty() && !has_execution_mismatches;

        Ok(CompatibilityReport {
            is_compatible,
            old_wasm_path: old_path.to_string(),
            new_wasm_path: new_path.to_string(),
            breaking_changes,
            non_breaking_changes,
            old_functions,
            new_functions,
            execution_diffs,
        })
    }

    /// Compute breaking and non-breaking changes between two sets of function signatures
    fn diff_signatures(
        _old: &[crate::utils::wasm::ContractFunctionSignature],
        _new: &[crate::utils::wasm::ContractFunctionSignature],
    ) -> (Vec<BreakingChange>, Vec<NonBreakingChange>) {
        // TODO: Implement signature diffing for ContractFunctionSignature
        (Vec::new(), Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_signatures_no_changes() {
        let analyzer = UpgradeAnalyzer::new();
        let sig = FunctionSignature {
            name: "test".to_string(),
            params: vec!["I32".to_string()],
            results: vec!["Val".to_string()],
        };
        let diff = analyzer.diff_signatures(std::slice::from_ref(&sig), std::slice::from_ref(&sig));
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert!(diff.changed.is_empty());
    }

    #[test]
    fn test_diff_signatures_add_remove() {
        let analyzer = UpgradeAnalyzer::new();
        let sig1 = FunctionSignature {
            name: "foo".into(),
            params: vec![],
            results: vec![],
        };
        let sig2 = FunctionSignature {
            name: "bar".into(),
            params: vec![],
            results: vec![],
        };

        let diff =
            analyzer.diff_signatures(std::slice::from_ref(&sig1), std::slice::from_ref(&sig2));

        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.removed[0].name, "foo");
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].name, "bar");
    }
}
