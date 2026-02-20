use crate::cli::args::{InspectArgs, InteractiveArgs, OptimizeArgs, RunArgs, UpgradeCheckArgs};
use crate::debugger::engine::DebuggerEngine;
use crate::logging;
use crate::repeat::RepeatRunner;
use crate::runtime::executor::ContractExecutor;
use crate::simulator::SnapshotLoader;
use crate::ui::tui::DebuggerUI;
use crate::Result;
use anyhow::Context;
use std::fs;

/// Execute the run command
pub fn run(args: RunArgs) -> Result<()> {
    logging::log_loading_contract(&args.contract.to_string_lossy());

    // Load WASM file
    let wasm_bytes = fs::read(&args.contract)
        .with_context(|| format!("Failed to read WASM file: {:?}", args.contract))?;

    logging::log_contract_loaded(wasm_bytes.len());

    // Load network snapshot if provided
    if let Some(snapshot_path) = &args.network_snapshot {
        logging::log_loading_snapshot(&snapshot_path.to_string_lossy());
        let loader = SnapshotLoader::from_file(snapshot_path)?;
        let loaded_snapshot = loader.apply_to_environment()?;
        logging::log_display(loaded_snapshot.format_summary(), logging::LogLevel::Info);
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

    let args_str = parsed_args.as_ref().map(|s| s.as_str());
    logging::log_execution_start(&args.function, args_str);

    // Create executor
    let mut executor = ContractExecutor::new(wasm_bytes)?;

    // Set up initial storage if provided
    if let Some(storage) = initial_storage {
        executor.set_initial_storage(storage)?;
    }

    // Create debugger engine
    let mut engine = DebuggerEngine::new(executor, args.breakpoint);

    // Execute with debugging
    let result = engine.execute(&args.function, parsed_args.as_deref())?;
    logging::log_execution_complete(&format!("{:?}", result));

    // Display events if requested
    if args.show_events {
        let events = engine.executor().get_events()?;
        let filtered_events = if let Some(topic) = &args.filter_topic {
            crate::inspector::events::EventInspector::filter_events(&events, topic)
        } else {
            events
        };

        if !filtered_events.is_empty() {
            for event in filtered_events.iter() {
                if let Some(contract_id) = &event.contract_id {
                    logging::log_event_emitted(contract_id, event.topics.len());
                }
            }
        }
    }

    // Display storage with optional filtering
    if !args.storage_filter.is_empty() {
        let storage_filter = crate::inspector::storage::StorageFilter::new(&args.storage_filter)
            .map_err(|e| anyhow::anyhow!("Invalid storage filter: {}", e))?;
        tracing::info!("Displaying filtered storage");
        let inspector = crate::inspector::StorageInspector::new();
        inspector.display_filtered(&storage_filter);
    }

    Ok(())
}

/// Execute the interactive command
pub fn interactive(args: InteractiveArgs) -> Result<()> {
    logging::log_loading_contract(&args.contract.to_string_lossy());

    // Load WASM file
    let wasm_bytes = fs::read(&args.contract)
        .with_context(|| format!("Failed to read WASM file: {:?}", args.contract))?;

    logging::log_contract_loaded(wasm_bytes.len());

    // Load network snapshot if provided
    if let Some(snapshot_path) = &args.network_snapshot {
        logging::log_loading_snapshot(&snapshot_path.to_string_lossy());
        let loader = SnapshotLoader::from_file(snapshot_path)?;
        let loaded_snapshot = loader.apply_to_environment()?;
        logging::log_display(loaded_snapshot.format_summary(), logging::LogLevel::Info);
    }

    // Create executor
    let executor = ContractExecutor::new(wasm_bytes)?;

    // Create debugger engine
    let engine = DebuggerEngine::new(executor, vec![]);

    // Start interactive UI
    logging::log_interactive_mode_start();

    let mut ui = DebuggerUI::new(engine)?;
    ui.run()?;

    Ok(())
}

/// Execute the inspect command
pub fn inspect(args: InspectArgs) -> Result<()> {
    logging::log_loading_contract(&args.contract.to_string_lossy());

    // Load WASM file
    let wasm_bytes = fs::read(&args.contract)
        .with_context(|| format!("Failed to read WASM file: {:?}", args.contract))?;

    logging::log_contract_loaded(wasm_bytes.len());

    if args.functions {
        let functions = crate::utils::wasm::parse_functions(&wasm_bytes)?;
        tracing::info!(count = functions.len(), "Exported functions found");
    }

    if args.metadata {
        tracing::debug!("Metadata parsing not yet implemented");
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
            tracing::debug!(count = arr.len(), "Parsed array arguments");
        }
        serde_json::Value::Object(ref obj) => {
            tracing::debug!(fields = obj.len(), "Parsed object arguments");
        }
        _ => {
            tracing::debug!("Parsed single value argument");
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
    logging::log_loading_contract(&args.contract.to_string_lossy());

    let wasm_bytes = fs::read(&args.contract)
        .with_context(|| format!("Failed to read WASM file: {:?}", args.contract))?;

    logging::log_contract_loaded(wasm_bytes.len());

    // Load network snapshot if provided
    if let Some(snapshot_path) = &args.network_snapshot {
        logging::log_loading_snapshot(&snapshot_path.to_string_lossy());
        let loader = SnapshotLoader::from_file(snapshot_path)?;
        let loaded_snapshot = loader.apply_to_environment()?;
        logging::log_display(loaded_snapshot.format_summary(), logging::LogLevel::Info);
    }

    let functions_to_analyze = if args.function.is_empty() {
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

    logging::log_analysis_start(&format!("gas optimization for {} functions", functions_to_analyze.len()));

    for function_name in &functions_to_analyze {
        match optimizer.analyze_function(function_name, args.args.as_deref()) {
            Ok(profile) => {
                tracing::debug!(function = function_name, cpu = profile.total_cpu, memory = profile.total_memory, "Function analyzed");
            }
            Err(e) => {
                tracing::warn!(function = function_name, error = %e, "Failed to analyze function");
            }
        }
    }

    let contract_path_str = args.contract.to_string_lossy().to_string();
    let report = optimizer.generate_report(&contract_path_str);

    let markdown = optimizer.generate_markdown_report(&report);

    if let Some(output_path) = &args.output {
        fs::write(output_path, &markdown)
            .with_context(|| format!("Failed to write report to: {:?}", output_path))?;
        logging::log_optimization_report(&output_path.to_string_lossy());
    } else {
        logging::log_display(&markdown, logging::LogLevel::Info);
    }

    Ok(())
}

/// Execute the upgrade-check command
pub fn upgrade_check(args: UpgradeCheckArgs) -> Result<()> {
    logging::log_contract_comparison(
        &args.old.to_string_lossy(),
        &args.new.to_string_lossy()
    );

    let old_bytes = fs::read(&args.old)
        .with_context(|| format!("Failed to read old WASM file: {:?}", args.old))?;
    let new_bytes = fs::read(&args.new)
        .with_context(|| format!("Failed to read new WASM file: {:?}", args.new))?;

    tracing::info!(old_size = old_bytes.len(), new_size = new_bytes.len(), "Loaded contracts for comparison");

    let analyzer = crate::analyzer::upgrade::UpgradeAnalyzer::new();

    logging::log_analysis_start("contract upgrade compatibility check");
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
        logging::log_optimization_report(&output_path.to_string_lossy());
    } else {
        logging::log_display(&markdown, logging::LogLevel::Info);
    }

    Ok(())
}
