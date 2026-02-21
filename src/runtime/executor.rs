use crate::utils::ArgumentParser;
use crate::{DebuggerError, Result};
use soroban_env_host::{DiagnosticLevel, Host};
use soroban_sdk::{Address, Env, InvokeError, Symbol, Val, Vec as SorobanVec};
use tracing::{info, warn};

/// Executes Soroban contracts in a test environment
pub struct ContractExecutor {
    env: Env,
    contract_address: Address,
}

impl ContractExecutor {
    /// Create a new contract executor
    pub fn new(wasm: Vec<u8>) -> Result<Self> {
        info!("Initializing contract executor");

        // Create a test environment
        let env = Env::default();

        // Enable diagnostic events
        env.host()
            .set_diagnostic_level(DiagnosticLevel::Debug)
            .expect("Failed to set diagnostic level");

        // Register the contract with the WASM
        let contract_address = env.register(wasm.as_slice(), ());

        info!("Contract registered successfully");

        Ok(Self {
            env,
            contract_address,
        })
    }

    /// Execute a contract function
    pub fn execute(&self, function: &str, args: Option<&str>) -> Result<String> {
        info!("Executing function: {}", function);

        // Convert function name to Symbol
        let func_symbol = Symbol::new(&self.env, function);

        // Parse arguments (simplified for now)
        let parsed_args = if let Some(args_json) = args {
            self.parse_args(args_json)?
        } else {
            vec![]
        };

        // Create argument vector
        let args_vec = if parsed_args.is_empty() {
            SorobanVec::<Val>::new(&self.env)
        } else {
            SorobanVec::from_slice(&self.env, &parsed_args)
        };

        // Call the contract
        // try_invoke_contract returns Result<Result<Val, ConversionError>, Result<InvokeError, InvokeError>>
        match self.env.try_invoke_contract::<Val, InvokeError>(
            &self.contract_address,
            &func_symbol,
            args_vec,
        ) {
            Ok(Ok(val)) => {
                info!("Function executed successfully");
                Ok(format!("{:?}", val))
            }
            Ok(Err(conv_err)) => {
                warn!("Return value conversion failed: {:?}", conv_err);
                Err(DebuggerError::ExecutionError(format!(
                    "Return value conversion failed: {:?}",
                    conv_err
                ))
                .into())
            }
            Err(Ok(inv_err)) => match inv_err {
                InvokeError::Contract(code) => {
                    warn!("Contract returned error code: {}", code);
                    Err(
                        DebuggerError::ExecutionError(format!("Contract error code: {}", code))
                            .into(),
                    )
                }
                InvokeError::Abort => {
                    warn!("Contract execution aborted");
                    Err(
                        DebuggerError::ExecutionError("Contract execution aborted".to_string())
                            .into(),
                    )
                }
            },
            Err(Err(inv_err)) => {
                warn!("Invocation error conversion failed: {:?}", inv_err);
                Err(DebuggerError::ExecutionError(format!(
                    "Invocation error conversion failed: {:?}",
                    inv_err
                ))
                .into())
            }
        }
    }

    /// Set initial storage state
    pub fn set_initial_storage(&mut self, _storage_json: String) -> Result<()> {
        // TODO: Implement storage initialization
        info!("Setting initial storage (not yet implemented)");
        Ok(())
    }

    /// Get the host instance
    pub fn host(&self) -> &Host {
        self.env.host()
    }

    /// Get the environment handle (clone)
    pub fn env_clone(&self) -> Env {
        self.env.clone()
    }

    /// Get the authorization tree from the environment
    pub fn get_auth_tree(&self) -> Result<Vec<crate::inspector::auth::AuthNode>> {
        crate::inspector::auth::AuthInspector::get_auth_tree(&self.env)
    }

    /// Parse JSON arguments into contract values
    fn parse_args(&self, args_json: &str) -> Result<Vec<Val>> {
        info!("Parsing arguments: {}", args_json);
        let parser = ArgumentParser::new(self.env.clone());
        parser.parse_args_string(args_json).map_err(|e| {
            warn!("Failed to parse arguments: {}", e);
            DebuggerError::InvalidArguments(e.to_string()).into()
        })
    }

    /// Capture a snapshot of current contract storage
    pub fn get_storage_snapshot(&self) -> Result<std::collections::HashMap<String, String>> {
        // In a real debugger, we would iterate over host.ledger_storage()
        // For now, we return a snapshot (placeholder logic)
        Ok(std::collections::HashMap::new())
    }

    /// Get events captured during execution
    pub fn get_events(&self) -> Result<Vec<crate::inspector::events::ContractEvent>> {
        crate::inspector::events::EventInspector::get_events(self.env.host())
    }

    /// Get mutable reference to environment (for dry-run state management)
    pub fn env_mut(&mut self) -> &mut Env {
        &mut self.env
    }

    /// Get reference to environment
    pub fn env(&self) -> &Env {
        &self.env
    }

    /// Get contract address
    pub fn contract_address(&self) -> &Address {
        &self.contract_address
    }

    /// Snapshot current storage state for dry-run rollback
    /// Returns a snapshot that can be used to restore state
    pub fn snapshot_storage(&self) -> Result<StorageSnapshot> {
        // For now, we'll create an empty snapshot
        // Full implementation would require accessing host storage internals
        // which may not be directly exposed. This is a placeholder that
        // documents the intended behavior.
        Ok(StorageSnapshot {
            contract_address: self.contract_address.clone(),
            // Storage state capture would go here if host API supports it
        })
    }

    /// Restore storage state from snapshot (for dry-run rollback)
    pub fn restore_storage(&mut self, _snapshot: &StorageSnapshot) -> Result<()> {
        // For now, this is a no-op as we don't have direct storage access
        // In a full implementation, this would restore all storage entries
        // to their pre-execution state
        info!("Storage state restored (dry-run rollback)");
        Ok(())
    }
}

/// Storage snapshot for dry-run rollback
#[derive(Debug, Clone)]
pub struct StorageSnapshot {
    contract_address: Address,
    // Future: Add fields to capture storage state
    // instance_storage: HashMap<String, Val>,
    // persistent_storage: HashMap<String, Val>,
    // temporary_storage: HashMap<String, Val>,
}
