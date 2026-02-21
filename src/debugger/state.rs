use crate::debugger::instruction_pointer::{InstructionPointer, StepMode};
use crate::runtime::instruction::Instruction;
use crate::inspector::stack::CallStackInspector;

/// Represents the current state of the debugger
#[derive(Debug, Clone)]
pub struct DebugState {
    current_function: Option<String>,
    step_count: usize,
    /// Instruction pointer for bytecode stepping
    instruction_pointer: InstructionPointer,
    /// Current instruction being executed
    current_instruction: Option<Instruction>,
    /// All parsed instructions for the current contract
    instructions: Vec<Instruction>,
    /// Whether instruction-level debugging is enabled
    instruction_debug_enabled: bool,
    call_stack: CallStackInspector,
}

impl DebugState {
    /// Create a new debug state
    pub fn new() -> Self {
        Self {
            current_function: None,
            step_count: 0,
            instruction_pointer: InstructionPointer::new(),
            current_instruction: None,
            instructions: Vec::new(),
            instruction_debug_enabled: false,
            call_stack: CallStackInspector::new(),
        }
    }

    /// Set the current function being executed
    pub fn set_current_function(&mut self, function: String) {
        self.current_function = Some(function);
    }

    /// Get the current function
    pub fn current_function(&self) -> Option<&str> {
        self.current_function.as_deref()
    }

    /// Increment step count
    pub fn increment_step(&mut self) {
        self.step_count += 1;
    }

    /// Get current step count
    pub fn step_count(&self) -> usize {
        self.step_count
    }

    /// Set the parsed instructions for the contract
    pub fn set_instructions(&mut self, instructions: Vec<Instruction>) {
        self.instructions = instructions;
        if !self.instructions.is_empty() {
            self.current_instruction = self.instructions.get(0).cloned();
        }
    }

    /// Get all instructions
    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    /// Get the current instruction
    pub fn current_instruction(&self) -> Option<&Instruction> {
        self.current_instruction.as_ref()
    }

    /// Get mutable reference to instruction pointer
    pub fn instruction_pointer_mut(&mut self) -> &mut InstructionPointer {
        &mut self.instruction_pointer
    }

    /// Get reference to instruction pointer
    pub fn instruction_pointer(&self) -> &InstructionPointer {
        &self.instruction_pointer
    }

    /// Enable instruction-level debugging
    pub fn enable_instruction_debug(&mut self) {
        self.instruction_debug_enabled = true;
    }

    /// Disable instruction-level debugging
    pub fn disable_instruction_debug(&mut self) {
        self.instruction_debug_enabled = false;
        self.instruction_pointer.stop_stepping();
    }

    /// Check if instruction-level debugging is enabled
    pub fn is_instruction_debug_enabled(&self) -> bool {
        self.instruction_debug_enabled
    }

    /// Start instruction stepping with given mode
    pub fn start_instruction_stepping(&mut self, mode: StepMode) {
        if self.instruction_debug_enabled {
            self.instruction_pointer.start_stepping(mode);
        }
    }

    /// Stop instruction stepping
    pub fn stop_instruction_stepping(&mut self) {
        self.instruction_pointer.stop_stepping();
    }

    /// Advance to the next instruction
    pub fn advance_to_instruction(&mut self, index: usize) -> Option<&Instruction> {
        if index < self.instructions.len() {
            self.instruction_pointer.advance_to(index);
            self.current_instruction = self.instructions.get(index).cloned();
            
            // Update call stack depth
            if let Some(ref inst) = self.current_instruction {
                self.instruction_pointer.update_call_stack(inst);
            }
            
            self.current_instruction.as_ref()
        } else {
            None
        }
    }

    /// Move to the next instruction
    pub fn next_instruction(&mut self) -> Option<&Instruction> {
        let next_index = self.instruction_pointer.current_index() + 1;
        self.advance_to_instruction(next_index)
    }

    /// Move to the previous instruction
    pub fn previous_instruction(&mut self) -> Option<&Instruction> {
        if let Some(prev_index) = self.instruction_pointer.step_back() {
            self.current_instruction = self.instructions.get(prev_index).cloned();
            self.current_instruction.as_ref()
        } else {
            None
        }
    }

    /// Check if we should pause execution at current instruction
    pub fn should_pause_execution(&self) -> bool {
        if !self.instruction_debug_enabled {
            return false;
        }
        
        if let Some(ref inst) = self.current_instruction {
            self.instruction_pointer.should_pause_at(inst)
        } else {
            false
        }
    }

    /// Get reference to call stack
    pub fn call_stack(&self) -> &CallStackInspector {
        &self.call_stack
    }

    /// Get mutable reference to call stack
    pub fn call_stack_mut(&mut self) -> &mut CallStackInspector {
        &mut self.call_stack
    }

    /// Reset the state
    pub fn reset(&mut self) {
        self.current_function = None;
        self.step_count = 0;
        self.instruction_pointer.reset();
        self.current_instruction = if !self.instructions.is_empty() {
            self.instructions.get(0).cloned()
        } else {
            None
        };
    }

    /// Get instruction context for display
    pub fn get_instruction_context(&self, context_size: usize) -> Vec<(usize, &Instruction, bool)> {
        let current_idx = self.instruction_pointer.current_index();
        let start = current_idx.saturating_sub(context_size);
        let end = (current_idx + context_size + 1).min(self.instructions.len());
        
        (start..end)
            .filter_map(|i| {
                self.instructions.get(i).map(|inst| (i, inst, i == current_idx))
            })
            .collect()
    }
}

impl Default for DebugState {
    fn default() -> Self {
        Self::new()
    }
}
