use crate::runtime::instruction::Instruction;
use crate::debugger::instruction_pointer::StepMode;
use crossterm::style::Stylize;

/// Pretty printing utilities for debugger output
pub struct Formatter;

impl Formatter {
    /// Format a value for display
    pub fn format_value(value: &str) -> String {
        // TODO: Add better formatting for different types
        value.to_string()
    }

    /// Format storage key-value pair
    pub fn format_storage_entry(key: &str, value: &str) -> String {
        format!("{} = {}", key, value)
    }

    /// Format a function call
    pub fn format_function_call(name: &str, args: Option<&str>) -> String {
        if let Some(args) = args {
            format!("{}({})", name, args)
        } else {
            format!("{}()", name)
        }
    }

    /// Format budget information
    pub fn format_budget(cpu: u64, cpu_limit: u64, mem: u64, mem_limit: u64) -> String {
        format!(
            "CPU: {}/{} ({:.1}%) | Memory: {}/{} bytes ({:.1}%)",
            cpu,
            cpu_limit,
            (cpu as f64 / cpu_limit as f64) * 100.0,
            mem,
            mem_limit,
            (mem as f64 / mem_limit as f64) * 100.0
        )
    }

    /// Format a single instruction for display
    pub fn format_instruction(instruction: &Instruction, is_current: bool) -> String {
        let prefix = if is_current { "►" } else { " " };
        let operands = instruction.operands();
        
        if operands.is_empty() {
            format!("{} {:08x}: {}", prefix, instruction.offset, instruction.name())
        } else {
            format!("{} {:08x}: {} {}", prefix, instruction.offset, instruction.name(), operands)
        }
    }

    /// Format instruction context with surrounding instructions
    pub fn format_instruction_context(
        context: &[(usize, Instruction, bool)],
        context_size: usize,
    ) -> String {
        let mut output = String::new();
        
        output.push_str("┌─ Instruction Context ─────────────────────────────┐\n");
        
        if context.is_empty() {
            output.push_str("│ No instructions available                         │\n");
        } else {
            for (idx, instruction, is_current) in context {
                let formatted = Self::format_instruction(instruction, *is_current);
                output.push_str(&format!("│ {:2}: {}│\n", idx, Self::pad_to_width(&formatted, 45)));
            }
        }
        
        output.push_str("└───────────────────────────────────────────────────┘\n");
        output
    }

    /// Format instruction pointer state
    pub fn format_instruction_pointer_state(
        current_index: usize,
        call_depth: u32,
        step_mode: Option<StepMode>,
        is_stepping: bool,
    ) -> String {
        let mut output = String::new();
        
        output.push_str("┌─ Instruction Pointer ─────────────────────────────┐\n");
        output.push_str(&format!("│ Current Index: {:5}                            │\n", current_index));
        output.push_str(&format!("│ Call Depth:    {:5}                            │\n", call_depth));
        
        if let Some(mode) = step_mode {
            let mode_str = match mode {
                StepMode::StepInto => "Step Into",
                StepMode::StepOver => "Step Over", 
                StepMode::StepOut => "Step Out",
                StepMode::StepBlock => "Step Block",
            };
            output.push_str(&format!("│ Step Mode:     {}                     │\n", Self::pad_to_width(mode_str, 15)));
        } else {
            output.push_str("│ Step Mode:     None                               │\n");
        }
        
        output.push_str(&format!("│ Stepping:      {}                              │\n", 
            if is_stepping { "Active " } else { "Inactive" }));
        output.push_str("└───────────────────────────────────────────────────┘\n");
        
        output
    }

    /// Format instruction statistics
    pub fn format_instruction_stats(
        total_instructions: usize,
        current_index: usize,
        instructions_executed: usize,
    ) -> String {
        let progress = if total_instructions > 0 {
            (current_index as f64 / total_instructions as f64) * 100.0
        } else {
            0.0
        };

        let mut output = String::new();
        output.push_str("┌─ Execution Progress ──────────────────────────────┐\n");
        output.push_str(&format!("│ Total Instructions:    {:8}                   │\n", total_instructions));
        output.push_str(&format!("│ Current Position:      {:8}                   │\n", current_index));
        output.push_str(&format!("│ Instructions Executed: {:8}                   │\n", instructions_executed));
        output.push_str(&format!("│ Progress:              {:6.1}%                  │\n", progress));
        
        // Progress bar
        let bar_width = 30;
        let filled = ((progress / 100.0) * bar_width as f64) as usize;
        let bar = "█".repeat(filled) + &"░".repeat(bar_width - filled);
        output.push_str(&format!("│ [{}] │\n", bar));
        
        output.push_str("└───────────────────────────────────────────────────┘\n");
        output
    }

    /// Format stepping help
    pub fn format_stepping_help() -> String {
        let mut output = String::new();
        output.push_str("┌─ Stepping Commands ───────────────────────────────┐\n");
        output.push_str("│ n, next     - Step to next instruction           │\n");
        output.push_str("│ s, step     - Step into calls                    │\n");
        output.push_str("│ o, over     - Step over calls                    │\n");
        output.push_str("│ u, out      - Step out of current function       │\n");
        output.push_str("│ b, block    - Step to next basic block           │\n");
        output.push_str("│ p, prev     - Step back to previous instruction  │\n");
        output.push_str("│ c, continue - Continue execution                 │\n");
        output.push_str("│ i, info     - Show instruction info              │\n");
        output.push_str("│ h, help     - Show this help                     │\n");
        output.push_str("└───────────────────────────────────────────────────┘\n");
        output
    }

    /// Pad string to specified width
    fn pad_to_width(s: &str, width: usize) -> String {
        if s.len() >= width {
            s.to_string()
        } else {
            format!("{}{}", s, " ".repeat(width - s.len()))
        }
    }

    /// Format an informational message in blue.
    pub fn info(message: impl AsRef<str>) -> String {
        Self::apply_color(message.as_ref(), ColorKind::Info)
    }

    /// Format a success message in green.
    pub fn success(message: impl AsRef<str>) -> String {
        Self::apply_color(message.as_ref(), ColorKind::Success)
    }

    /// Format a warning message in yellow.
    pub fn warning(message: impl AsRef<str>) -> String {
        Self::apply_color(message.as_ref(), ColorKind::Warning)
    }

    /// Format an error message in red.
    pub fn error(message: impl AsRef<str>) -> String {
        Self::apply_color(message.as_ref(), ColorKind::Error)
    }

    /// Configure whether ANSI colors are enabled.
    pub fn configure_colors(enable: bool) {
        COLOR_ENABLED.store(enable, std::sync::atomic::Ordering::Relaxed);
    }

    /// Auto-configure color output based on environment.
    /// If NO_COLOR is set, colors are disabled.
    pub fn configure_colors_from_env() {
        let no_color = std::env::var_os("NO_COLOR").is_some();
        Self::configure_colors(!no_color);
    }

    fn apply_color(message: &str, kind: ColorKind) -> String {
        if !COLOR_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            return message.to_string();
        }

        match kind {
            ColorKind::Info => format!("{}", message.blue()),
            ColorKind::Success => format!("{}", message.green()),
            ColorKind::Warning => format!("{}", message.yellow()),
            ColorKind::Error => format!("{}", message.red()),
        }
    }
}

#[derive(Copy, Clone)]
enum ColorKind {
    Info,
    Success,
    Warning,
    Error,
}

static COLOR_ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
