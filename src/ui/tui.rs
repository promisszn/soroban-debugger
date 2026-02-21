use crate::debugger::engine::DebuggerEngine;
use crate::inspector::{BudgetInspector, StorageInspector};
use crate::Result;
use std::io::{self, Write};

/// Terminal user interface for interactive debugging
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

    /// Get mutable reference to storage inspector
    pub fn storage_inspector_mut(&mut self) -> &mut StorageInspector {
        &mut self.storage_inspector
    }

    /// Run the interactive UI loop
    pub fn run(&mut self) -> Result<()> {
        self.print_help();

        loop {
            print!("\n(debug) ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

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
                    eprintln!("Error: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Handle a single command
    fn handle_command(&mut self, command: &str) -> Result<bool> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(false);
        }

        match parts[0] {
            "run" => {
                if parts.len() < 2 {
                    println!("Usage: run <function_name> [args_json]");
                } else {
                    let function = parts[1];
                    let args = if parts.len() > 2 {
                        Some(parts[2..].join(" "))
                    } else {
                        None
                    };
                    println!("\n--- Execution Start: {} ---", function);
                    match self.engine.execute(function, args.as_deref()) {
                        Ok(result) => {
                            if self.engine.is_paused() {
                                self.render_breakpoint_hit();
                            }
                            println!("\n--- Execution Complete ---");
                            println!("Result: {:?}", result);
                        }
                        Err(e) => {
                            println!("\n--- Execution Failed ---");
                            println!("Error: {}", e);
                            self.engine.state().call_stack().display();
                        }
                    }
                }
            }
            "s" | "step" => {
                self.engine.step()?;
                println!("Stepped");
            }
            "c" | "continue" => {
                self.engine.continue_execution()?;
                println!("Continuing...");
            }
            "i" | "inspect" => {
                self.inspect();
            }
            "storage" => {
                self.storage_inspector.display();
            }
            "stack" => {
                self.engine.state().call_stack().display();
            }
            "budget" => {
                BudgetInspector::display(self.engine.executor().host());
            }
            "break" => {
                if parts.len() < 2 {
                    println!("Usage: break <function_name>");
                } else {
                    self.engine.breakpoints_mut().add(parts[1]);
                    println!("Breakpoint set at: {}", parts[1]);
                }
            }
            "list-breaks" => {
                let breakpoints = self.engine.breakpoints_mut().list();
                if breakpoints.is_empty() {
                    println!("No breakpoints set");
                } else {
                    println!("Breakpoints:");
                    for bp in breakpoints {
                        println!("  - {}", bp);
                    }
                }
            }
            "clear" => {
                if parts.len() < 2 {
                    println!("Usage: clear <function_name>");
                } else if self.engine.breakpoints_mut().remove(parts[1]) {
                    println!("Breakpoint removed: {}", parts[1]);
                } else {
                    println!("No breakpoint at: {}", parts[1]);
                }
            }
            "help" => {
                self.print_help();
            }
            "q" | "quit" | "exit" => {
                println!("Exiting debugger");
                return Ok(true);
            }
            _ => {
                println!(
                    "Unknown command: {}. Type 'help' for available commands.",
                    parts[0]
                );
            }
        }

        Ok(false)
    }

    /// Render a pretty breakpoint hit display
    fn render_breakpoint_hit(&self) {
        let state = self.engine.state();
        let current_func = state.current_function().unwrap_or("unknown");
        let args = state.current_args().unwrap_or("none");
        let stack = state.call_stack().get_stack();
        
        // Find previous frame if it exists
        let prev_func = if stack.len() > 1 {
            stack[stack.len() - 2].function.as_str()
        } else {
            "none"
        };

        println!("\n┌────────────────────────────────────────────────────────────────────────┐");
        println!("│ 🛑 BREAKPOINT HIT                                                      │");
        println!("├────────────────────────────────────────────────────────────────────────┤");
        println!("│ {:<14} │ {:<53} │", "Function", current_func);
        println!("│ {:<14} │ {:<53} │", "Arguments", args);
        println!("│ {:<14} │ {:<53} │", "Previous", prev_func);
        println!("├────────────────────────────────────────────────────────────────────────┤");
        println!("│ STORAGE STATE                                                          │");
        
        let storage = self.storage_inspector.get_all();
        if storage.is_empty() {
            println!("│ (empty)                                                                │");
        } else {
            let mut keys: Vec<&String> = storage.keys().collect();
            keys.sort();
            for key in keys.iter().take(5) { // Show first 5 entries
                let val = &storage[*key];
                let entry = format!("{} = {}", key, val);
                println!("│ {:<70} │", if entry.len() > 68 { format!("{}...", &entry[..65]) } else { entry });
            }
            if storage.len() > 5 {
                println!("│ ... (and {} more)                                                     │", storage.len() - 5);
            }
        }
        println!("└────────────────────────────────────────────────────────────────────────┘");
    }

    /// Display current state
    fn inspect(&self) {
        if self.engine.is_paused() {
            self.render_breakpoint_hit();
        } else {
            println!("\n=== Current State ===");
            if let Some(func) = self.engine.state().current_function() {
                println!("Function: {}", func);
            } else {
                println!("Function: (none)");
            }
            println!("Steps: {}", self.engine.state().step_count());
            println!("Paused: {}", self.engine.is_paused());

            println!();
            self.engine.state().call_stack().display();
        }
    }

    /// Print help message
    fn print_help(&self) {
        println!("\nAvailable commands:");
        println!("  run <func> [args]    Run a contract function");
        println!("  s, step              Execute next instruction");
        println!("  c, continue          Run until breakpoint or completion");
        println!("  i, inspect           Show current execution state");
        println!("  storage              Display contract storage");
        println!("  stack                Show call stack");
        println!("  budget               Show resource usage (CPU/memory)");
        println!("  break <function>     Set breakpoint at function");
        println!("  list-breaks          List all breakpoints");
        println!("  clear <function>     Remove breakpoint");
        println!("  help                 Show this help message");
        println!("  q, quit              Exit debugger");
    }
}
