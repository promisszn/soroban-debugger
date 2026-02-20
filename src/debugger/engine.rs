use crate::debugger::breakpoint::BreakpointManager;
use crate::debugger::state::DebugState;
use crate::debugger::stepper::Stepper;
use crate::debugger::instruction_pointer::StepMode;
use crate::runtime::executor::ContractExecutor;
use crate::runtime::instrumentation::{Instrumenter, InstructionHook};
use crate::runtime::instruction::Instruction;
use crate::Result;
use tracing::info;
use std::sync::{Arc, Mutex};

/// Core debugging engine that orchestrates execution and debugging
pub struct DebuggerEngine {
    executor: ContractExecutor,
    breakpoints: BreakpointManager,
    state: Arc<Mutex<DebugState>>,
    stepper: Stepper,
    instrumenter: Instrumenter,
    paused: bool,
    instruction_debug_enabled: bool,
}

impl DebuggerEngine {
    /// Create a new debugger engine
    pub fn new(executor: ContractExecutor, initial_breakpoints: Vec<String>) -> Self {
        let mut breakpoints = BreakpointManager::new();

        // Add initial breakpoints
        for bp in initial_breakpoints {
            breakpoints.add(&bp);
            info!("Breakpoint set at function: {}", bp);
        }

        Self {
            executor,
            breakpoints,
            state: Arc::new(Mutex::new(DebugState::new())),
            stepper: Stepper::new(),
            instrumenter: Instrumenter::new(),
            paused: false,
            instruction_debug_enabled: false,
        }
    }

    /// Enable instruction-level debugging
    pub fn enable_instruction_debug(&mut self, wasm_bytes: &[u8]) -> Result<()> {
        info!("Enabling instruction-level debugging");
        
        // Parse instructions from WASM
        let instructions = self.instrumenter.parse_instructions(wasm_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to parse instructions: {}", e))?;
        
        let instructions_len = instructions.len();
        
        // Update debug state with instructions
        if let Ok(mut state) = self.state.lock() {
            state.set_instructions(instructions.to_vec());
            state.enable_instruction_debug();
        }
        
        // Enable instrumentation
        self.instrumenter.enable();
        
        // Set up instruction hook
        let state_ref = Arc::clone(&self.state);
        self.instrumenter.set_hook(move |instruction_index: usize, _instruction: &Instruction| {
            // This callback will be called for each instruction during execution
            if let Ok(mut state) = state_ref.lock() {
                // Advance to the current instruction
                state.advance_to_instruction(instruction_index);
                
                // Check if we should pause
                state.should_pause_execution()
            } else {
                false
            }
        });
        
        self.instruction_debug_enabled = true;
        info!("Instruction-level debugging enabled with {} instructions", instructions_len);
        
        Ok(())
    }

    /// Disable instruction-level debugging
    pub fn disable_instruction_debug(&mut self) {
        info!("Disabling instruction-level debugging");
        
        self.instrumenter.disable();
        self.instrumenter.remove_hook();
        
        if let Ok(mut state) = self.state.lock() {
            state.disable_instruction_debug();
        }
        
        self.instruction_debug_enabled = false;
    }

    /// Check if instruction-level debugging is enabled
    pub fn is_instruction_debug_enabled(&self) -> bool {
        self.instruction_debug_enabled
    }

    /// Execute a contract function with debugging
    pub fn execute(&mut self, function: &str, args: Option<&str>) -> Result<String> {
        info!("Executing function: {}", function);

        // Check if we should break at function entry
        if self.breakpoints.should_break(function) {
            self.pause_at_function(function);
        }

        // Set current function in state
        if let Ok(mut state) = self.state.lock() {
            state.set_current_function(function.to_string());
        }

        // Execute the contract
        let result = self.executor.execute(function, args)?;

        info!("Execution completed");
        Ok(result)
    }

    /// Step into next instruction
    pub fn step_into(&mut self) -> Result<bool> {
        info!("Step into instruction");
        
        if !self.instruction_debug_enabled {
            return Err(anyhow::anyhow!("Instruction debugging not enabled"));
        }

        let stepped = if let Ok(mut state) = self.state.lock() {
            self.stepper.step_into(&mut state)
        } else {
            false
        };

        self.paused = stepped;
        Ok(stepped)
    }

    /// Step over function calls
    pub fn step_over(&mut self) -> Result<bool> {
        info!("Step over instruction");
        
        if !self.instruction_debug_enabled {
            return Err(anyhow::anyhow!("Instruction debugging not enabled"));
        }

        let stepped = if let Ok(mut state) = self.state.lock() {
            self.stepper.step_over(&mut state)
        } else {
            false
        };

        self.paused = stepped;
        Ok(stepped)
    }

    /// Step out of current function
    pub fn step_out(&mut self) -> Result<bool> {
        info!("Step out of function");
        
        if !self.instruction_debug_enabled {
            return Err(anyhow::anyhow!("Instruction debugging not enabled"));
        }

        let stepped = if let Ok(mut state) = self.state.lock() {
            self.stepper.step_out(&mut state)
        } else {
            false
        };

        self.paused = stepped;
        Ok(stepped)
    }

    /// Step to next basic block
    pub fn step_block(&mut self) -> Result<bool> {
        info!("Step to next basic block");
        
        if !self.instruction_debug_enabled {
            return Err(anyhow::anyhow!("Instruction debugging not enabled"));
        }

        let stepped = if let Ok(mut state) = self.state.lock() {
            self.stepper.step_block(&mut state)
        } else {
            false
        };

        self.paused = stepped;
        Ok(stepped)
    }

    /// Step backwards to previous instruction
    pub fn step_back(&mut self) -> Result<bool> {
        info!("Step back to previous instruction");
        
        if !self.instruction_debug_enabled {
            return Err(anyhow::anyhow!("Instruction debugging not enabled"));
        }

        let stepped = if let Ok(mut state) = self.state.lock() {
            self.stepper.step_back(&mut state)
        } else {
            false
        };

        self.paused = stepped;
        Ok(stepped)
    }

    /// Start instruction stepping with given mode
    pub fn start_instruction_stepping(&mut self, mode: StepMode) -> Result<()> {
        if !self.instruction_debug_enabled {
            return Err(anyhow::anyhow!("Instruction debugging not enabled"));
        }

        if let Ok(mut state) = self.state.lock() {
            self.stepper.start(mode, &mut state);
            self.paused = true;
        }

        Ok(())
    }

    /// Continue execution until next breakpoint
    pub fn continue_execution(&mut self) -> Result<()> {
        info!("Continuing execution...");
        self.paused = false;
        
        if let Ok(mut state) = self.state.lock() {
            self.stepper.continue_execution(&mut state);
        }
        
        Ok(())
    }

    /// Pause execution at a function
    fn pause_at_function(&mut self, function: &str) {
        println!("\n[BREAKPOINT] Paused at function: {}", function);
        self.paused = true;
        
        if let Ok(mut state) = self.state.lock() {
            state.set_current_function(function.to_string());
        }
    }

    /// Check if debugger is currently paused
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Get current debug state
    pub fn state(&self) -> Arc<Mutex<DebugState>> {
        Arc::clone(&self.state)
    }

    /// Get current instruction
    pub fn current_instruction(&self) -> Option<Instruction> {
        if let Ok(state) = self.state.lock() {
            state.current_instruction().cloned()
        } else {
            None
        }
    }

    /// Get instruction context for display
    pub fn get_instruction_context(&self, context_size: usize) -> Vec<(usize, Instruction, bool)> {
        if let Ok(state) = self.state.lock() {
            state.get_instruction_context(context_size)
                .into_iter()
                .map(|(idx, inst, current)| (idx, inst.clone(), current))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get mutable reference to breakpoint manager
    pub fn breakpoints_mut(&mut self) -> &mut BreakpointManager {
        &mut self.breakpoints
    }

    /// Get reference to executor
    pub fn executor(&self) -> &ContractExecutor {
        &self.executor
    }

    /// Compatibility method for old step interface
    pub fn step(&mut self) -> Result<()> {
        let _ = self.step_into()?;
        Ok(())
    }
}
