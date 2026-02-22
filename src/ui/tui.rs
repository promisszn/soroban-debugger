use crate::debugger::engine::DebuggerEngine;
use crate::inspector::{BudgetInspector, StorageInspector};
use crate::Result;
use std::io::{self, Write};

/// Terminal user interface for interactive debugging.
pub struct DebuggerUI {
    engine: DebuggerEngine,
    storage_inspector: StorageInspector,
}

impl DebuggerUI {
    pub fn new(engine: DebuggerEngine) -> Result<Self> {
        Ok(Self {
            engine,
            storage_inspector: StorageInspector::new(),
        })
    }

    /// Run the interactive UI loop.
    pub fn run(&mut self) -> Result<()> {
        self.print_help();

        loop {
            print!("\n(debug) ");
            io::stdout().flush().map_err(|e| {
                crate::DebuggerError::FileError(format!("Failed to flush stdout: {}", e))
            })?;

            let mut input = String::new();
            io::stdin().read_line(&mut input).map_err(|e| {
                crate::DebuggerError::FileError(format!("Failed to read line: {}", e))
            })?;

            let command = input.trim();
            if command.is_empty() {
                continue;
            }

            match self.handle_command(command) {
                Ok(should_exit) => {
                    if should_exit {
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "Command execution error");
                }
            }
        }

        Ok(())
    }

    fn handle_command(&mut self, command: &str) -> Result<bool> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(false);
        }

        match parts[0] {
            "s" | "step" => {
                self.engine.step()?;
                if let Ok(state) = self.engine.state().lock() {
                    crate::logging::log_step(state.step_count() as u64);
                }
            }
            "c" | "continue" => {
                self.engine.continue_execution()?;
                tracing::info!("Execution continuing");
            }
            "i" | "inspect" => {
                self.inspect();
            }
            "storage" => {
                self.storage_inspector.display();
            }
            "stack" => {
                if let Ok(state) = self.engine.state().lock() {
                    state.call_stack().display();
                }
            }
            "budget" => {
                BudgetInspector::display(self.engine.executor().host());
            }
            "break" => {
                if parts.len() < 2 {
                    tracing::warn!("breakpoint set without function name");
                } else {
                    self.engine.breakpoints_mut().add(parts[1]);
                    crate::logging::log_breakpoint_set(parts[1]);
                }
            }
            "list-breaks" => {
                let breakpoints = self.engine.breakpoints_mut().list();
                if breakpoints.is_empty() {
                    crate::logging::log_display(
                        "No breakpoints set",
                        crate::logging::LogLevel::Info,
                    );
                } else {
                    for bp in breakpoints {
                        crate::logging::log_display(
                            format!("- {}", bp),
                            crate::logging::LogLevel::Info,
                        );
                    }
                }
            }
            "clear" => {
                if parts.len() < 2 {
                    tracing::warn!("clear command missing function name");
                } else if self.engine.breakpoints_mut().remove(parts[1]) {
                    crate::logging::log_breakpoint_cleared(parts[1]);
                } else {
                    tracing::debug!(breakpoint = parts[1], "No breakpoint found at function");
                }
            }
            "help" => self.print_help(),
            "q" | "quit" | "exit" => {
                tracing::info!("Exiting debugger");
                return Ok(true);
            }
            _ => tracing::warn!(command = parts[0], "Unknown command"),
        }

        Ok(false)
    }

    fn inspect(&self) {
        crate::logging::log_display("\n=== Current State ===", crate::logging::LogLevel::Info);
        if let Ok(state) = self.engine.state().lock() {
            if let Some(func) = state.current_function() {
                crate::logging::log_display(
                    format!("Function: {}", func),
                    crate::logging::LogLevel::Info,
                );
            } else {
                crate::logging::log_display("Function: (none)", crate::logging::LogLevel::Info);
            }
            crate::logging::log_display(
                format!("Steps: {}", state.step_count()),
                crate::logging::LogLevel::Info,
            );
            crate::logging::log_display(
                format!("Paused: {}", self.engine.is_paused()),
                crate::logging::LogLevel::Info,
            );
            crate::logging::log_display("", crate::logging::LogLevel::Info);
            state.call_stack().display();
        } else {
            crate::logging::log_display("State unavailable", crate::logging::LogLevel::Info);
        }
    }

    fn print_help(&self) {
        crate::logging::log_display(
            "Interactive debugger commands:",
            crate::logging::LogLevel::Info,
        );
        crate::logging::log_display(
            "  step | s           Step execution",
            crate::logging::LogLevel::Info,
        );
        crate::logging::log_display(
            "  continue | c       Continue execution",
            crate::logging::LogLevel::Info,
        );
        crate::logging::log_display(
            "  inspect | i        Show current state",
            crate::logging::LogLevel::Info,
        );
        crate::logging::log_display(
            "  storage            Show tracked storage view",
            crate::logging::LogLevel::Info,
        );
        crate::logging::log_display(
            "  stack              Show call stack",
            crate::logging::LogLevel::Info,
        );
        crate::logging::log_display(
            "  budget             Show budget usage",
            crate::logging::LogLevel::Info,
        );
        crate::logging::log_display(
            "  break <func>       Set breakpoint",
            crate::logging::LogLevel::Info,
        );
        crate::logging::log_display(
            "  list-breaks        List breakpoints",
            crate::logging::LogLevel::Info,
        );
        crate::logging::log_display(
            "  clear <func>       Clear breakpoint",
            crate::logging::LogLevel::Info,
        );
        crate::logging::log_display(
            "  help               Show this help",
            crate::logging::LogLevel::Info,
        );
        crate::logging::log_display(
            "  quit | q           Exit debugger",
            crate::logging::LogLevel::Info,
        );
    }
}
