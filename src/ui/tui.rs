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
            "sb" | "step-back" => {
                let stepped = self.engine.step_back()?;
                if stepped {
                    println!("Stepped back");
                    self.inspect();
                } else {
                    println!("Already at beginning of history");
                }
            }
            "cb" | "continue-back" => {
                self.engine.continue_back()?;
                println!("Continued back");
                self.inspect();
            }
            "goto" => {
                if parts.len() < 2 {
                    println!("Usage: goto <step>");
                } else if let Ok(step) = parts[1].parse::<usize>() {
                    self.engine.goto_step(step)?;
                    println!("Jumped to step {}", step);
                    self.inspect();
                } else {
                    println!("Invalid step number");
                }
            }
            "timeline" | "tl" => {
                let timeline = self.engine.get_timeline();
                println!("\n=== Execution Timeline ===");
                for (i, snap) in timeline.get_history().iter().enumerate() {
                    let current = if i == timeline.current_pos() { "▶" } else { " " };
                    println!(
                        "{} {:>3}: Step {:>3} | fn: {:<15} | IP: {:>3}",
                        current, i, snap.step, snap.function, snap.instruction_index
                    );
                }
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
                    println!("No breakpoints set");
                } else {
                    for bp in breakpoints {
                        println!("- {}", bp);
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
        println!("\n=== Current State ===");
        if let Ok(state) = self.engine.state().lock() {
            if let Some(func) = state.current_function() {
                println!("Function: {}", func);
            } else {
                println!("Function: (none)");
            }
            println!("Steps: {}", state.step_count());
            println!("Paused: {}", self.engine.is_paused());
            println!();
            state.call_stack().display();
        } else {
            println!("State unavailable");
        }
    }

    fn print_help(&self) {
        println!("Interactive debugger commands:");
        println!("  step | s           Step execution");
        println!("  step-back | sb     Step backward in time");
        println!("  continue | c       Continue execution");
        println!("  continue-back | cb Continue execution backwards");
        println!("  goto <step>        Jump to specific step");
        println!("  timeline | tl      Show execution timeline");
        println!("  inspect | i        Show current state");
        println!("  storage            Show tracked storage view");
        println!("  stack              Show call stack");
        println!("  budget             Show budget usage");
        println!("  break <func>       Set breakpoint");
        println!("  list-breaks        List breakpoints");
        println!("  clear <func>       Clear breakpoint");
        println!("  help               Show this help");
        println!("  quit | q           Exit debugger");
    }
}
