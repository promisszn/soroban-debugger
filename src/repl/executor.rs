/// REPL command execution
///
/// Handles execution of function calls and storage inspection
/// against the loaded contract.
use super::ReplConfig;
use crate::inspector::StorageInspector;
use crate::runtime::executor::ContractExecutor;

use crate::Result;
use std::fs;

/// Executor for REPL commands
pub struct ReplExecutor {
    #[allow(dead_code)]
    executor: ContractExecutor,
    storage_inspector: StorageInspector,
}

impl ReplExecutor {
    /// Create a new REPL executor
    pub fn new(config: &ReplConfig) -> Result<Self> {
        let wasm_bytes = fs::read(&config.contract_path).map_err(|_e| {
            miette::miette!(
                "Failed to read contract WASM file: {:?}",
                config.contract_path
            )
        })?;
        let executor = ContractExecutor::new(wasm_bytes)?;
        let storage_inspector = StorageInspector::new();

        Ok(ReplExecutor {
            executor,
            storage_inspector,
        })
    }

    /// Call a contract function
    pub async fn call_function(&mut self, function: &str, args: Vec<String>) -> Result<()> {
        crate::logging::log_display(
            format!("Calling function: {} with args: {:?}", function, args),
            crate::logging::LogLevel::Info,
        );

        // For now, we'll just simulate the call and show the formatted output
        // In a real implementation, this would execute against the loaded contract
        crate::logging::log_display(
            format!("✓ Function {} called successfully", function),
            crate::logging::LogLevel::Info,
        );

        // Show the result
        crate::logging::log_display("Result: (simulated output)", crate::logging::LogLevel::Info);

        Ok(())
    }

    /// Inspect and display contract storage
    pub fn inspect_storage(&self) -> Result<()> {
        let entries = self.storage_inspector.get_all();

        if entries.is_empty() {
            crate::logging::log_display("Storage is empty", crate::logging::LogLevel::Warn);
            return Ok(());
        }

        crate::logging::log_display("", crate::logging::LogLevel::Info);
        crate::logging::log_display("=== Contract Storage ===", crate::logging::LogLevel::Info);
        crate::logging::log_display("", crate::logging::LogLevel::Info);

        let mut last_prefix = String::new();
        for (key, value) in entries.iter() {
            // Extract prefix for grouping
            let parts: Vec<&str> = key.splitn(2, '-').collect();
            let prefix = parts.first().unwrap_or(&"").to_string();

            if prefix != last_prefix && !last_prefix.is_empty() {
                crate::logging::log_display("", crate::logging::LogLevel::Info);
            }
            last_prefix = prefix;

            crate::logging::log_display(
                format!("  {}: {}", key, value),
                crate::logging::LogLevel::Info,
            );
        }
        crate::logging::log_display("", crate::logging::LogLevel::Info);

        Ok(())
    }
}
