use crate::cli::args::{InspectArgs, InteractiveArgs, OptimizeArgs, RunArgs, UpgradeCheckArgs};
use crate::debugger::engine::DebuggerEngine;
use crate::debugger::instruction_pointer::StepMode;
use crate::repeat::RepeatRunner;
use crate::runtime::executor::ContractExecutor;
use crate::simulator::SnapshotLoader;
use crate::ui::tui::DebuggerUI;
use crate::ui::formatter::Formatter;
use crate::Result;
use anyhow::Context;
use std::fs;
use tracing::info as log_info;

/// Execute the run command
pub fn run(args: RunArgs) -> Result<()> {
    println!("Loading contract: {:?}", args.contract);

    // Load WASM file
    let wasm_bytes = fs::read(&args.contract)
        .with_context(|| format!("Failed to read WASM file: {:?}", args.contract))?;

    println!("Contract loaded successfully ({} bytes)", wasm_bytes.len());

    // Load network snapshot if provided
    if let Some(snapshot_path) = &args.network_snapshot {
        println!("\nLoading network snapshot: {:?}", snapshot_path);
        let loader = SnapshotLoader::from_file(snapshot_path)?;
        let loaded_snapshot = loader.apply_to_environment()?;
        println!("{}", loaded_snapshot.format_summary());
    }

    // Parse arguments if provided
    let parsed_args = if let Some(args_json) = &args.args {
        Some(parse_args(args_json)?)
    } else {
        None
    };

    // Parse storage if provided
    let initial_storage = if let Some(storage_json) = &args.storage {
        Some(parse_storage(storage_json)?)
    } else {
        None
    };

    // Handle --repeat N: run N times and show aggregate stats
    if let Some(n) = args.repeat {
        let runner = RepeatRunner::new(wasm_bytes, args.breakpoint, initial_storage);
        let stats = runner.run(&args.function, parsed_args.as_deref(), n)?;
        stats.display();
        return Ok(());
    }

    println!("\nStarting debugger...");
    println!("Function: {}", args.function);
    if let Some(ref args) = parsed_args {
        println!("Arguments: {}", args);
    }

    // Create executor
    let mut executor = ContractExecutor::new(wasm_bytes.clone())?;

    // Set up initial storage if provided
    if let Some(storage) = initial_storage {
        executor.set_initial_storage(storage)?;
    }

    // Create debugger engine
    let mut engine = DebuggerEngine::new(executor, args.breakpoint);

    // Enable instruction-level debugging if requested
    if args.instruction_debug {
        println!("Enabling instruction-level debugging...");
        engine.enable_instruction_debug(&wasm_bytes)?;
        
        // Parse step mode
        let step_mode = match args.step_mode.to_lowercase().as_str() {
            "into" => StepMode::StepInto,
            "over" => StepMode::StepOver,
            "out" => StepMode::StepOut,
            "block" => StepMode::StepBlock,
            _ => {
                println!("Warning: Invalid step mode '{}', using 'into'", args.step_mode);
                StepMode::StepInto
            }
        };

        // Start instruction stepping if requested
        if args.step_instructions {
            println!("Starting instruction stepping in {} mode", args.step_mode);
            engine.start_instruction_stepping(step_mode)?;
            
            // Enter instruction stepping mode
            run_instruction_stepping(&mut engine, &args.function, parsed_args.as_deref())?;
            return Ok(());
        }
    }

    // Execute with debugging
    println!("\n--- Execution Start ---\n");
    let result = engine.execute(&args.function, parsed_args.as_deref())?;
    println!("\n--- Execution Complete ---\n");

    println!("Result: {:?}", result);

    // Display events if requested
    if args.show_events {
        println!("\n--- Events ---");
        let events = engine.executor().get_events()?;
        let filtered_events = if let Some(topic) = &args.filter_topic {
            crate::inspector::events::EventInspector::filter_events(&events, topic)
        } else {
            events
        };

        if filtered_events.is_empty() {
            println!("No events captured.");
        } else {
            for (i, event) in filtered_events.iter().enumerate() {
                println!("Event #{}:", i);
                if let Some(contract_id) = &event.contract_id {
                    println!("  Contract: {}", contract_id);
                }
                println!("  Topics: {:?}", event.topics);
                println!("  Data: {}", event.data);
                println!();
            }
        }
    }

    // Display storage with optional filtering
    if !args.storage_filter.is_empty() {
        let storage_filter = crate::inspector::storage::StorageFilter::new(&args.storage_filter)
            .map_err(|e| anyhow::anyhow!("Invalid storage filter: {}", e))?;
        println!("\n--- Storage ---");
        let inspector = crate::inspector::StorageInspector::new();
        inspector.display_filtered(&storage_filter);
    }

    Ok(())
}

/// Execute the interactive command
pub fn interactive(args: InteractiveArgs) -> Result<()> {
    println!("Starting interactive debugger for: {:?}", args.contract);

    // Load WASM file
    let wasm_bytes = fs::read(&args.contract)
        .with_context(|| format!("Failed to read WASM file: {:?}", args.contract))?;

    println!("Contract loaded successfully ({} bytes)", wasm_bytes.len());

    // Load network snapshot if provided
    if let Some(snapshot_path) = &args.network_snapshot {
        println!("\nLoading network snapshot: {:?}", snapshot_path);
        let loader = SnapshotLoader::from_file(snapshot_path)?;
        let loaded_snapshot = loader.apply_to_environment()?;
        println!("{}", loaded_snapshot.format_summary());
    }

    // Create executor
    let executor = ContractExecutor::new(wasm_bytes)?;

    // Create debugger engine
    let engine = DebuggerEngine::new(executor, vec![]);

    // Start interactive UI
    println!("\nStarting interactive mode...");
    println!("Type 'help' for available commands\n");

    let mut ui = DebuggerUI::new(engine)?;
    ui.run()?;

    Ok(())
}

/// Execute the inspect command
pub fn inspect(args: InspectArgs) -> Result<()> {
    println!("Inspecting contract: {:?}", args.contract);

    // Load WASM file
    let wasm_bytes = fs::read(&args.contract)
        .with_context(|| format!("Failed to read WASM file: {:?}", args.contract))?;

    println!("\nContract Information:");
    println!("  Size: {} bytes", wasm_bytes.len());

    if args.functions {
        println!("\nExported Functions:");
        let functions = crate::utils::wasm::parse_functions(&wasm_bytes)?;
        for func in functions {
            println!("  - {}", func);
        }
    }

    if args.metadata {
        println!("\nMetadata:");
        println!("  (Metadata parsing not yet implemented)");
    }

    Ok(())
}

/// Parse JSON arguments with validation (actual parsing happens during execution)
pub fn parse_args(json: &str) -> Result<String> {
    // Validate JSON structure at parse time to give quick feedback
    let value = serde_json::from_str::<serde_json::Value>(json)
        .with_context(|| format!("Invalid JSON arguments: {}", json))?;

    // Provide helpful context about what was parsed
    match value {
        serde_json::Value::Array(ref arr) => {
            log_info!("Parsed {} argument(s)", arr.len());
        }
        serde_json::Value::Object(ref obj) => {
            log_info!("Parsed object with {} fields for Map argument", obj.len());
        }
        _ => {
            log_info!("Parsed single value argument");
        }
    }

    Ok(json.to_string())
}

/// Parse JSON storage into a string for now (will be improved later)
pub fn parse_storage(json: &str) -> Result<String> {
    // Basic validation
    serde_json::from_str::<serde_json::Value>(json)
        .with_context(|| format!("Invalid JSON storage: {}", json))?;
    Ok(json.to_string())
}

/// Execute the optimize command
pub fn optimize(args: OptimizeArgs) -> Result<()> {
    println!(
        "Analyzing contract for gas optimization: {:?}",
        args.contract
    );

    let wasm_bytes = fs::read(&args.contract)
        .with_context(|| format!("Failed to read WASM file: {:?}", args.contract))?;

    println!("Contract loaded successfully ({} bytes)", wasm_bytes.len());

    // Load network snapshot if provided
    if let Some(snapshot_path) = &args.network_snapshot {
        println!("\nLoading network snapshot: {:?}", snapshot_path);
        let loader = SnapshotLoader::from_file(snapshot_path)?;
        let loaded_snapshot = loader.apply_to_environment()?;
        println!("{}", loaded_snapshot.format_summary());
    }

    let functions_to_analyze = if args.function.is_empty() {
        println!("No functions specified, analyzing all exported functions...");
        crate::utils::wasm::parse_functions(&wasm_bytes)?
    } else {
        args.function.clone()
    };

    let mut executor = ContractExecutor::new(wasm_bytes)?;

    if let Some(storage_json) = &args.storage {
        let storage = parse_storage(storage_json)?;
        executor.set_initial_storage(storage)?;
    }

    let mut optimizer = crate::profiler::analyzer::GasOptimizer::new(executor);

    println!("\nAnalyzing {} function(s)...", functions_to_analyze.len());

    for function_name in &functions_to_analyze {
        println!("  Analyzing function: {}", function_name);
        match optimizer.analyze_function(function_name, args.args.as_deref()) {
            Ok(profile) => {
                println!(
                    "    CPU: {} instructions, Memory: {} bytes",
                    profile.total_cpu, profile.total_memory
                );
            }
            Err(e) => {
                eprintln!(
                    "    Warning: Failed to analyze function {}: {}",
                    function_name, e
                );
            }
        }
    }

    let contract_path_str = args.contract.to_string_lossy().to_string();
    let report = optimizer.generate_report(&contract_path_str);

    let markdown = optimizer.generate_markdown_report(&report);

    if let Some(output_path) = &args.output {
        fs::write(output_path, &markdown)
            .with_context(|| format!("Failed to write report to: {:?}", output_path))?;
        println!("\nOptimization report written to: {:?}", output_path);
    } else {
        println!("\n{}", markdown);
    }

    Ok(())
}

/// Execute the upgrade-check command
pub fn upgrade_check(args: UpgradeCheckArgs) -> Result<()> {
    println!("Comparing contracts...");
    println!("  Old: {:?}", args.old);
    println!("  New: {:?}", args.new);

    let old_bytes = fs::read(&args.old)
        .with_context(|| format!("Failed to read old WASM file: {:?}", args.old))?;
    let new_bytes = fs::read(&args.new)
        .with_context(|| format!("Failed to read new WASM file: {:?}", args.new))?;

    println!(
        "Loaded contracts (Old: {} bytes, New: {} bytes)",
        old_bytes.len(),
        new_bytes.len()
    );

    let analyzer = crate::analyzer::upgrade::UpgradeAnalyzer::new();

    println!("Running analysis...");
    let report = analyzer.analyze(
        &old_bytes,
        &new_bytes,
        args.function.as_deref(),
        args.args.as_deref(),
    )?;

    let markdown = analyzer.generate_markdown_report(&report);

    if let Some(output_path) = &args.output {
        fs::write(output_path, &markdown)
            .with_context(|| format!("Failed to write report to: {:?}", output_path))?;
        println!("\nCompatibility report written to: {:?}", output_path);
    } else {
        println!("\n{}", markdown);
    }

    Ok(())
}

/// Run instruction-level stepping mode
fn run_instruction_stepping(
    engine: &mut DebuggerEngine,
    function: &str,
    args: Option<&str>,
) -> Result<()> {
    println!("\n=== Instruction Stepping Mode ===");
    println!("Type 'help' for available commands\n");

    // Display initial instruction context
    display_instruction_context(engine, 3);

    loop {
        print!("(step) > ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let input = input.trim().to_lowercase();

        match input.as_str() {
            "n" | "next" => {
                if let Ok(stepped) = engine.step_into() {
                    if stepped {
                        println!("Stepped to next instruction");
                        display_instruction_context(engine, 3);
                    } else {
                        println!("Cannot step: execution finished or error occurred");
                    }
                } else {
                    println!("Error stepping: instruction debugging not enabled");
                }
            }
            "s" | "step" | "into" => {
                if let Ok(stepped) = engine.step_into() {
                    if stepped {
                        println!("Stepped into next instruction");
                        display_instruction_context(engine, 3);
                    } else {
                        println!("Cannot step into: execution finished or error occurred");
                    }
                } else {
                    println!("Error stepping: instruction debugging not enabled");
                }
            }
            "o" | "over" => {
                if let Ok(stepped) = engine.step_over() {
                    if stepped {
                        println!("Stepped over instruction");
                        display_instruction_context(engine, 3);
                    } else {
                        println!("Cannot step over: execution finished or error occurred");
                    }
                } else {
                    println!("Error stepping: instruction debugging not enabled");
                }
            }
            "u" | "out" => {
                if let Ok(stepped) = engine.step_out() {
                    if stepped {
                        println!("Stepped out of function");
                        display_instruction_context(engine, 3);
                    } else {
                        println!("Cannot step out: execution finished or error occurred");
                    }
                } else {
                    println!("Error stepping: instruction debugging not enabled");
                }
            }
            "b" | "block" => {
                if let Ok(stepped) = engine.step_block() {
                    if stepped {
                        println!("Stepped to next basic block");
                        display_instruction_context(engine, 3);
                    } else {
                        println!("Cannot step to next block: execution finished or error occurred");
                    }
                } else {
                    println!("Error stepping: instruction debugging not enabled");
                }
            }
            "p" | "prev" | "back" => {
                if let Ok(stepped) = engine.step_back() {
                    if stepped {
                        println!("Stepped back to previous instruction");
                        display_instruction_context(engine, 3);
                    } else {
                        println!("Cannot step back: no previous instruction");
                    }
                } else {
                    println!("Error stepping: instruction debugging not enabled");
                }
            }
            "c" | "continue" => {
                println!("Continuing execution...");
                engine.continue_execution()?;
                
                // Execute the function
                let result = engine.execute(function, args)?;
                println!("Execution completed. Result: {:?}", result);
                break;
            }
            "i" | "info" => {
                display_instruction_info(engine);
            }
            "ctx" | "context" => {
                print!("Enter context size (default 5): ");
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
                let mut size_input = String::new();
                std::io::stdin().read_line(&mut size_input).unwrap();
                let size = size_input.trim().parse().unwrap_or(5);
                display_instruction_context(engine, size);
            }
            "h" | "help" => {
                println!("{}", Formatter::format_stepping_help());
            }
            "q" | "quit" | "exit" => {
                println!("Exiting instruction stepping mode...");
                break;
            }
            "" => {
                // Repeat last command (step into)
                if let Ok(stepped) = engine.step_into() {
                    if stepped {
                        println!("Stepped to next instruction");
                        display_instruction_context(engine, 3);
                    } else {
                        println!("Cannot step: execution finished or error occurred");
                    }
                } else {
                    println!("Error stepping: instruction debugging not enabled");
                }
            }
            _ => {
                println!("Unknown command: {}. Type 'help' for available commands.", input);
            }
        }
    }

    Ok(())
}

/// Display instruction context around current position
fn display_instruction_context(engine: &DebuggerEngine, context_size: usize) {
    let context = engine.get_instruction_context(context_size);
    let formatted = Formatter::format_instruction_context(&context, context_size);
    println!("{}", formatted);
}

/// Display detailed instruction information
fn display_instruction_info(engine: &DebuggerEngine) {
    if let Some(state) = engine.state().lock().ok() {
        let ip = state.instruction_pointer();
        let step_mode = if ip.is_stepping() { Some(ip.step_mode()) } else { None };
        
        println!("{}", Formatter::format_instruction_pointer_state(
            ip.current_index(),
            ip.call_stack_depth(),
            step_mode,
            ip.is_stepping(),
        ));

        let stats = Formatter::format_instruction_stats(
            state.instructions().len(),
            ip.current_index(),
            state.step_count(),
        );
        println!("{}", stats);

        if let Some(current_inst) = state.current_instruction() {
            println!("Current Instruction Details:");
            println!("  Name: {}", current_inst.name());
            println!("  Offset: 0x{:08x}", current_inst.offset);
            println!("  Function: {}", current_inst.function_index);
            println!("  Local Index: {}", current_inst.local_index);
            println!("  Operands: {}", current_inst.operands());
            println!("  Control Flow: {}", current_inst.is_control_flow());
            println!("  Function Call: {}", current_inst.is_call());
        }
    } else {
        println!("Cannot access debug state");
    }
}
