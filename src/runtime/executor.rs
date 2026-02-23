use crate::runtime::mocking::MockRegistry;
use crate::utils::ArgumentParser;
use crate::{runtime::mocking::MockCallLogEntry, runtime::mocking::MockContractDispatcher};
use crate::{DebuggerError, Result};

use indicatif::{ProgressBar, ProgressStyle};
use soroban_env_host::xdr::ScVal;
use soroban_env_host::{DiagnosticLevel, Host, TryFromVal};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, InvokeError, Symbol, Val, Vec as SorobanVec};

use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

/// Represents a captured execution trace.
#[derive(Debug, Clone)]
pub struct ExecutionRecord {
    pub function: String,
    pub args: Vec<ScVal>,
    pub result: std::result::Result<ScVal, String>,
    pub storage_before: HashMap<String, String>,
    pub storage_after: HashMap<String, String>,
}

/// Storage snapshot for dry-run rollback.
#[derive(Clone)]
pub struct StorageSnapshot {
    pub storage: soroban_env_host::storage::Storage,
}

use crate::debugger::error_db::ErrorDatabase;

/// Executes Soroban contracts in a test environment.
pub struct ContractExecutor {
 impl ContractExecutor {
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
        );
        spinner.set_message(format!("Executing function: {}...", function));
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));

        // Validate function existence
        let exported_functions = match crate::utils::wasm::parse_functions(&self.wasm_bytes) {
            Ok(funcs) => funcs,
            Err(e) => {
                spinner.finish_and_clear();
                return Err(e);
            }
        };
        if !exported_functions.contains(&function.to_string()) {
            spinner.finish_and_clear();
            return Err(DebuggerError::InvalidFunction(function.to_string()).into());
        }

        // Convert function name to Symbol
        let func_symbol = Symbol::new(&self.env, function);

        let parsed_args = if let Some(args_json) = args {
            match self.parse_args(function, args_json) {
                Ok(args) => args,
                Err(e) => {
                    spinner.finish_and_clear();
                    return Err(e);
                }
            }
        } else {
            vec![]
        };

        let args_vec = if parsed_args.is_empty() {
            SorobanVec::<Val>::new(&self.env)
        } else {
            SorobanVec::from_slice(&self.env, &parsed_args)
        };

        // Capture storage before
        let storage_before = match self.get_storage_snapshot() {
            Ok(snapshot) => snapshot,
            Err(e) => {
                spinner.finish_and_clear();
                return Err(e);
            }
        };

 impl ContractExecutor {
                *storage = snapshot.storage.clone();
                Ok(())
            })
            .map_err(|e| {
                DebuggerError::ExecutionError(format!("Failed to restore storage: {:?}", e))
            })?;
        info!("Storage state restored (dry-run rollback)");
        Ok(())
    }

    /// Get diagnostic events from the host.
    pub fn get_diagnostic_events(&self) -> Result<Vec<soroban_env_host::xdr::ContractEvent>> {
        Ok(self
            .env
            .host()
            .get_diagnostic_events()
            .map_err(|e| {
                DebuggerError::ExecutionError(format!("Failed to get diagnostic events: {}", e))
            })?
            .0
            .into_iter()
            .map(|he| he.event)
            .collect())
    }

    fn parse_args(&self, function: &str, args_json: &str) -> Result<Vec<Val>> {
        let parser = ArgumentParser::new(self.env.clone());
        let normalized_args_json = self.normalize_args_for_function(function, args_json)?;

        parser
            .parse_args_string(&normalized_args_json)
            .map_err(|e| {
                warn!("Failed to parse arguments: {}", e);
                DebuggerError::InvalidArguments(e.to_string()).into()
            })
    }

    fn normalize_args_for_function(&self, function: &str, args_json: &str) -> Result<String> {
        let signatures = crate::utils::wasm::parse_function_signatures(&self.wasm_bytes)?;
        let Some(signature) = signatures.into_iter().find(|sig| sig.name == function) else {
            return Ok(args_json.to_string());
        };

        let mut args_value: JsonValue = serde_json::from_str(args_json).map_err(|e| {
            DebuggerError::InvalidArguments(format!("Invalid JSON in --args: {}", e))
        })?;

        let JsonValue::Array(args) = &mut args_value else {
            return Ok(args_json.to_string());
        };

        for (arg, param) in args.iter_mut().zip(signature.params.iter()) {
            if param.type_name.starts_with("Option<") {
                if is_typed_annotation(arg) {
                    continue;
                }
                *arg = serde_json::json!({"type": "option", "value": arg.clone()});
                continue;
            }

            if param.type_name.starts_with("Tuple<") {
                let arity = tuple_arity_from_type_name(&param.type_name).ok_or_else(|| {
                    DebuggerError::InvalidArguments(format!(
                        "Invalid tuple type in function spec for '{}': {}",
                        param.name, param.type_name
                    ))
                })?;

                let JsonValue::Array(actual_arr) = arg else {
                    return Err(DebuggerError::InvalidArguments(format!(
                        "Argument '{}' expects tuple with {} elements, got {}",
                        param.name,
                        arity,
                        json_type_name(arg)
                    ))
                    .into());
                };

                if actual_arr.len() != arity {
                    return Err(DebuggerError::InvalidArguments(format!(
                        "Tuple arity mismatch: expected {}, got {}",
                        arity,
                        actual_arr.len()
                    ))
                    .into());
                }

                *arg = serde_json::json!({"type": "tuple", "arity": arity, "value": actual_arr.clone()});
            }
        }

        serde_json::to_string(&args_value).map_err(|e| {
            DebuggerError::ExecutionError(format!("Failed to normalize arguments JSON: {}", e))
                .into()
        })
    }

    fn install_mock_dispatchers(&self) -> Result<()> {
        let ids = match self.mock_registry.lock() {
            Ok(registry) => registry.mocked_contract_ids(),
            Err(_) => {
                return Err(DebuggerError::ExecutionError(
                    "Mock registry lock poisoned".to_string(),
                )
                .into())
            }
        };

        for contract_id in ids {
            let address = self.parse_contract_address(&contract_id)?;
            let dispatcher =
                MockContractDispatcher::new(contract_id.clone(), Arc::clone(&self.mock_registry))
                    .boxed();
            self.env
                .host()
                .register_test_contract(address.to_object(), dispatcher)
                .map_err(|e| {
                    DebuggerError::ExecutionError(format!(
                        "Failed to register test contract: {}",
                        e
                    ))
                })?;
        }

        Ok(())
    }
    fn parse_contract_address(&self, contract_id: &str) -> Result<Address> {
        let parsed = catch_unwind(AssertUnwindSafe(|| {
            Address::from_str(&self.env, contract_id)
        }));
        match parsed {
            Ok(addr) => Ok(addr),
            Err(_) => Err(DebuggerError::InvalidArguments(format!(
                "Invalid contract id in --mock: {contract_id}"
            ))
            .into()),
        }
    }
}

fn tuple_arity_from_type_name(type_name: &str) -> Option<usize> {
    let inner = type_name.strip_prefix("Tuple<")?.strip_suffix('>')?;
    if inner.trim().is_empty() {
        return Some(0);
    }

    let mut depth = 0usize;
    let mut arity = 1usize;
    for ch in inner.chars() {
        match ch {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => arity += 1,
            _ => {}
        }
    }

    Some(arity)
}

fn is_typed_annotation(value: &JsonValue) -> bool {
    matches!(
        value,
        JsonValue::Object(obj) if obj.get("type").is_some() && obj.get("value").is_some()
    )
}

fn json_type_name(value: &JsonValue) -> &'static str {
    match value {
        JsonValue::Null => "null",
        JsonValue::Bool(_) => "boolean",
        JsonValue::Number(_) => "number",
        JsonValue::String(_) => "string",
        JsonValue::Array(_) => "array",
        JsonValue::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::tuple_arity_from_type_name;

    #[test]
    fn tuple_arity_counts_top_level_types() {
        assert_eq!(tuple_arity_from_type_name("Tuple<U32, Symbol>"), Some(2));
        assert_eq!(
            tuple_arity_from_type_name("Tuple<U32, Option<Vec<Symbol>>, Map<U32, String>>"),
            Some(3)
        );
    }
}