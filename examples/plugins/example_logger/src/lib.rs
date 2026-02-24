use soroban_debugger::plugin::{
    EventContext, ExecutionEvent, InspectorPlugin, PluginCapabilities, PluginCommand,
    PluginError, PluginManifest, PluginResult,
};
use std::any::Any;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

/// Example logger plugin that logs execution events to a file
pub struct ExampleLoggerPlugin {
    manifest: PluginManifest,
    log_file: Mutex<Option<PathBuf>>,
    event_count: Mutex<usize>,
}

impl ExampleLoggerPlugin {
    fn new() -> Self {
        let manifest = PluginManifest {
            name: "example-logger".to_string(),
            version: "1.0.0".to_string(),
            description: "Example plugin that logs execution events to a file".to_string(),
            author: "Soroban Debugger Contributors".to_string(),
            license: Some("MIT OR Apache-2.0".to_string()),
            min_debugger_version: Some("0.1.0".to_string()),
            capabilities: PluginCapabilities {
                hooks_execution: true,
                provides_commands: true,
                provides_formatters: false,
                supports_hot_reload: true,
            },
            library: "libexample_logger_plugin.dylib".to_string(),
            dependencies: vec![],
        };

        Self {
            manifest,
            log_file: Mutex::new(None),
            event_count: Mutex::new(0),
        }
    }

    fn log_event(&self, message: &str) {
        if let Ok(log_file_guard) = self.log_file.lock() {
            if let Some(ref path) = *log_file_guard {
                if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    
                    let _ = writeln!(file, "[{}] {}", timestamp, message);
                }
            }
        }
    }
}

impl InspectorPlugin for ExampleLoggerPlugin {
    fn metadata(&self) -> PluginManifest {
        self.manifest.clone()
    }

    fn initialize(&mut self) -> PluginResult<()> {
        // Set default log file path
        let home = dirs::home_dir().ok_or_else(|| {
            PluginError::InitializationFailed("Could not determine home directory".to_string())
        })?;
        
        let log_path = home.join(".soroban-debug").join("plugin-logs").join("example-logger.log");
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                PluginError::InitializationFailed(format!("Failed to create log directory: {}", e))
            })?;
        }
        
        *self.log_file.lock().unwrap() = Some(log_path.clone());
        
        self.log_event(&format!("Plugin initialized. Logging to: {:?}", log_path));
        
        Ok(())
    }

    fn shutdown(&mut self) -> PluginResult<()> {
        let count = *self.event_count.lock().unwrap();
        self.log_event(&format!("Plugin shutting down. Total events processed: {}", count));
        Ok(())
    }

    fn on_event(&mut self, event: &ExecutionEvent, context: &mut EventContext) -> PluginResult<()> {
        // Increment event counter
        if let Ok(mut count) = self.event_count.lock() {
            *count += 1;
        }

        // Log different event types
        match event {
            ExecutionEvent::BeforeFunctionCall { function, args } => {
                self.log_event(&format!(
                    "BEFORE_CALL: {} with args: {:?} (depth: {})",
                    function,
                    args,
                    context.stack_depth
                ));
            }
            ExecutionEvent::AfterFunctionCall { function, result, duration } => {
                let status = match result {
                    Ok(_) => "SUCCESS",
                    Err(_) => "ERROR",
                };
                self.log_event(&format!(
                    "AFTER_CALL: {} - {} (duration: {:?})",
                    function,
                    status,
                    duration
                ));
            }
            ExecutionEvent::BeforeInstruction { pc, instruction } => {
                self.log_event(&format!(
                    "BEFORE_INSTRUCTION: PC={} Instruction={}",
                    pc,
                    instruction
                ));
            }
            ExecutionEvent::AfterInstruction { pc, instruction } => {
                self.log_event(&format!(
                    "AFTER_INSTRUCTION: PC={} Instruction={}",
                    pc,
                    instruction
                ));
            }
            ExecutionEvent::BreakpointHit { function, condition } => {
                self.log_event(&format!(
                    "BREAKPOINT: {} (condition: {:?})",
                    function,
                    condition
                ));
            }
            ExecutionEvent::ExecutionPaused { reason } => {
                self.log_event(&format!("PAUSED: {}", reason));
            }
            ExecutionEvent::ExecutionResumed => {
                self.log_event("RESUMED");
            }
            ExecutionEvent::StorageAccess { operation, key, value } => {
                self.log_event(&format!(
                    "STORAGE: {:?} key={} value={:?}",
                    operation,
                    key,
                    value
                ));
            }
            ExecutionEvent::DiagnosticEvent { contract_id, topics, data } => {
                self.log_event(&format!(
                    "DIAGNOSTIC: contract={:?} topics={:?} data={}",
                    contract_id,
                    topics,
                    data
                ));
            }
            ExecutionEvent::Error { message, context: error_context } => {
                self.log_event(&format!(
                    "ERROR: {} (context: {:?})",
                    message,
                    error_context
                ));
            }
        }

        Ok(())
    }

    fn commands(&self) -> Vec<PluginCommand> {
        vec![
            PluginCommand {
                name: "log-stats".to_string(),
                description: "Show logging statistics".to_string(),
                arguments: vec![],
            },
            PluginCommand {
                name: "log-path".to_string(),
                description: "Show the log file path".to_string(),
                arguments: vec![],
            },
            PluginCommand {
                name: "clear-log".to_string(),
                description: "Clear the log file".to_string(),
                arguments: vec![],
            },
        ]
    }

    fn execute_command(&mut self, command: &str, _args: &[String]) -> PluginResult<String> {
        match command {
            "log-stats" => {
                let count = *self.event_count.lock().unwrap();
                Ok(format!("Total events logged: {}", count))
            }
            "log-path" => {
                if let Ok(log_file_guard) = self.log_file.lock() {
                    if let Some(ref path) = *log_file_guard {
                        Ok(format!("Log file: {:?}", path))
                    } else {
                        Ok("Log file not initialized".to_string())
                    }
                } else {
                    Err(PluginError::ExecutionFailed("Failed to access log file".to_string()))
                }
            }
            "clear-log" => {
                if let Ok(log_file_guard) = self.log_file.lock() {
                    if let Some(ref path) = *log_file_guard {
                        std::fs::write(path, "").map_err(|e| {
                            PluginError::ExecutionFailed(format!("Failed to clear log: {}", e))
                        })?;
                        Ok("Log file cleared".to_string())
                    } else {
                        Err(PluginError::ExecutionFailed("Log file not initialized".to_string()))
                    }
                } else {
                    Err(PluginError::ExecutionFailed("Failed to access log file".to_string()))
                }
            }
            _ => Err(PluginError::ExecutionFailed(format!("Unknown command: {}", command))),
        }
    }

    fn supports_hot_reload(&self) -> bool {
        true
    }

    fn prepare_reload(&self) -> PluginResult<Box<dyn Any + Send>> {
        let count = *self.event_count.lock().unwrap();
        let log_file = self.log_file.lock().unwrap().clone();
        Ok(Box::new((count, log_file)))
    }

    fn restore_from_reload(&mut self, state: Box<dyn Any + Send>) -> PluginResult<()> {
        if let Ok((count, log_file)) = state.downcast::<(usize, Option<PathBuf>)>() {
            *self.event_count.lock().unwrap() = *count;
            *self.log_file.lock().unwrap() = *log_file;
            self.log_event("Plugin reloaded successfully");
            Ok(())
        } else {
            Err(PluginError::ExecutionFailed("Failed to restore state".to_string()))
        }
    }
}

/// Plugin constructor function that the debugger will call to create the plugin instance
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn InspectorPlugin {
    let plugin = ExampleLoggerPlugin::new();
    Box::into_raw(Box::new(plugin))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let plugin = ExampleLoggerPlugin::new();
        assert_eq!(plugin.metadata().name, "example-logger");
        assert_eq!(plugin.commands().len(), 3);
        assert!(plugin.supports_hot_reload());
    }

    #[test]
    fn test_plugin_commands() {
        let plugin = ExampleLoggerPlugin::new();
        let commands = plugin.commands();
        
        assert!(commands.iter().any(|c| c.name == "log-stats"));
        assert!(commands.iter().any(|c| c.name == "log-path"));
        assert!(commands.iter().any(|c| c.name == "clear-log"));
    }
}
