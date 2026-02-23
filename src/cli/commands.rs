use crate::analyzer::upgrade::{CompatibilityReport, ExecutionDiff, UpgradeAnalyzer};
use crate::cli::args::{InspectArgs, InteractiveArgs, RunArgs, UpgradeCheckArgs};
use crate::cli::args::{
    AnalyzeArgs, CompareArgs, GraphFormat, InspectArgs, InteractiveArgs, OptimizeArgs, ProfileArgs,
    RemoteArgs, ReplArgs, ReplayArgs, RunArgs, ScenarioArgs, ServerArgs, SymbolicArgs, TuiArgs,
    UpgradeCheckArgs, Verbosity,
};
use crate::debugger::engine::DebuggerEngine;
use crate::debugger::instruction_pointer::StepMode;
use crate::history::{check_regression, HistoryManager, RunHistory};
use crate::logging;
use crate::output::OutputConfig;
use crate::repeat::RepeatRunner;
use crate::runtime::executor::ContractExecutor;
use crate::simulator::SnapshotLoader;
use crate::ui::formatter::Formatter;
use crate::ui::tui::DebuggerUI;
use crate::{DebuggerError, Result};
use miette::WrapErr;
use serde::Serialize;
use std::fs;
use std::io::Write;
use textplots::{Chart, Plot, Shape};

fn print_info(message: impl AsRef<str>) {
    if !Formatter::is_quiet() {
        println!("{}", Formatter::info(message));
    }
}

fn print_success(message: impl AsRef<str>) {
    if !Formatter::is_quiet() {
        println!("{}", Formatter::success(message));
    }
}

fn print_warning(message: impl AsRef<str>) {
    if !Formatter::is_quiet() {
        println!("{}", Formatter::warning(message));
    }
}

/// Print the final contract return value — always shown regardless of verbosity.
fn print_result(message: impl AsRef<str>) {
    println!("{}", Formatter::success(message));
}

/// Print verbose-only detail — only shown when --verbose is active.
fn print_verbose(message: impl AsRef<str>) {
    if Formatter::is_verbose() {
        println!("{}", Formatter::info(message));
    }
}

/// Execute batch mode with parallel execution
fn run_batch(args: &RunArgs, batch_file: &std::path::Path) -> Result<()> {
    print_info(format!("Loading contract: {:?}", args.contract));
    logging::log_loading_contract(&args.contract.to_string_lossy());

    let wasm_bytes = fs::read(&args.contract).map_err(|e| {
        DebuggerError::WasmLoadError(format!(
            "Failed to read WASM file at {:?}: {}",
            args.contract, e
        ))
    })?;

    print_success(format!(
        "Contract loaded successfully ({} bytes)",
        wasm_bytes.len()
    ));
    logging::log_contract_loaded(wasm_bytes.len());

    print_info(format!("Loading batch file: {:?}", batch_file));
    let batch_items = crate::batch::BatchExecutor::load_batch_file(batch_file)?;
    print_success(format!("Loaded {} test cases", batch_items.len()));

    if let Some(snapshot_path) = &args.network_snapshot {
        print_info(format!("\nLoading network snapshot: {:?}", snapshot_path));
        logging::log_loading_snapshot(&snapshot_path.to_string_lossy());
        let loader = SnapshotLoader::from_file(snapshot_path)?;
        let loaded_snapshot = loader.apply_to_environment()?;
        logging::log_display(loaded_snapshot.format_summary(), logging::LogLevel::Info);
    }

    print_info(format!(
        "\nExecuting {} test cases in parallel for function: {}",
        batch_items.len(),
        args.function
    ));
    logging::log_execution_start(&args.function, None);

    let executor = crate::batch::BatchExecutor::new(wasm_bytes, args.function.clone());
    let results = executor.execute_batch(batch_items)?;
    let summary = crate::batch::BatchExecutor::summarize(&results);

    crate::batch::BatchExecutor::display_results(&results, &summary);

    if args.json
        || args
            .format
            .as_deref()
            .map(|f| f.eq_ignore_ascii_case("json"))
            .unwrap_or(false)
    {
        let output = serde_json::json!({
            "results": results,
            "summary": summary,
        });
        logging::log_display(
            serde_json::to_string_pretty(&output).map_err(|e| {
                DebuggerError::FileError(format!("Failed to serialize output: {}", e))
            })?,
            logging::LogLevel::Info,
        );
    }

    logging::log_execution_complete(&format!("{}/{} passed", summary.passed, summary.total));

    if summary.failed > 0 || summary.errors > 0 {
        return Err(DebuggerError::ExecutionError(format!(
            "Batch execution completed with failures: {} failed, {} errors",
            summary.failed, summary.errors
        ))
        .into());
    }

    Ok(())
}

/// Execute the run command.
#[tracing::instrument(skip_all, fields(contract = ?args.contract, function = args.function))]
pub fn run(args: RunArgs, verbosity: Verbosity) -> Result<()> {
    // Initialize output writer
    let mut output_writer = OutputWriter::new(args.save_output.as_ref(), args.append)?;

    // Handle batch execution mode
    if let Some(batch_file) = &args.batch_args {
        return run_batch(&args, batch_file);
    }

    if args.dry_run {
        return run_dry_run(&args);
    }

    print_info(format!("Loading contract: {:?}", args.contract));
    output_writer.write(&format!("Loading contract: {:?}", args.contract))?;
    logging::log_loading_contract(&args.contract.to_string_lossy());

    let wasm_file = crate::utils::wasm::load_wasm(&args.contract)
        .with_context(|| format!("Failed to read WASM file: {:?}", args.contract))?;
    let wasm_bytes = wasm_file.bytes;
    let wasm_hash = wasm_file.sha256_hash;

    if let Some(expected) = &args.expected_hash {
        if expected.to_lowercase() != wasm_hash {
            return Err(crate::DebuggerError::ChecksumMismatch {
                expected: expected.clone(),
                actual: wasm_hash.clone(),
            }
            .into());
        }
    }

    print_success(format!(
        "Contract loaded successfully ({} bytes)",
        wasm_bytes.len()
    ));
    output_writer.write(&format!(
        "Contract loaded successfully ({} bytes)",
        wasm_bytes.len()
    ))?;

    if args.verbose || verbosity == Verbosity::Verbose {
        print_verbose(format!("SHA-256: {}", wasm_hash));
        output_writer.write(&format!("SHA-256: {}", wasm_hash))?;
        if args.expected_hash.is_some() {
            print_verbose("Checksum verified ✓");
            output_writer.write("Checksum verified ✓")?;
        }
    }

    logging::log_contract_loaded(wasm_bytes.len());

    if let Some(snapshot_path) = &args.network_snapshot {
        print_info(format!("\nLoading network snapshot: {:?}", snapshot_path));
        output_writer.write(&format!("Loading network snapshot: {:?}", snapshot_path))?;
        logging::log_loading_snapshot(&snapshot_path.to_string_lossy());
        let loader = SnapshotLoader::from_file(snapshot_path)?;
        let loaded_snapshot = loader.apply_to_environment()?;
        output_writer.write(&loaded_snapshot.format_summary())?;
        logging::log_display(loaded_snapshot.format_summary(), logging::LogLevel::Info);
    }

    let parsed_args = if let Some(args_json) = &args.args {
        Some(parse_args(args_json)?)
    } else {
        None
    };

    let mut initial_storage = if let Some(storage_json) = &args.storage {
        Some(parse_storage(storage_json)?)
    } else {
        None
    };

    // Import storage if specified
    if let Some(import_path) = &args.import_storage {
        print_info(format!("Importing storage from: {:?}", import_path));
        let imported = crate::inspector::storage::StorageState::import_from_file(import_path)?;
        print_success(format!("Imported {} storage entries", imported.len()));
        initial_storage = Some(serde_json::to_string(&imported).map_err(|e| {
            DebuggerError::StorageError(format!("Failed to serialize imported storage: {}", e))
        })?);
    }

    if let Some(n) = args.repeat {
        logging::log_repeat_execution(&args.function, n as usize);
        let runner = RepeatRunner::new(wasm_bytes, args.breakpoint, initial_storage);
        let stats = runner.run(&args.function, parsed_args.as_deref(), n)?;
        stats.display();
        return Ok(());
    }

    print_info("\nStarting debugger...");
    output_writer.write("Starting debugger...")?;
    print_info(format!("Function: {}", args.function));
    output_writer.write(&format!("Function: {}", args.function))?;
    if let Some(ref parsed) = parsed_args {
        print_info(format!("Arguments: {}", parsed));
        output_writer.write(&format!("Arguments: {}", parsed))?;
    }
    logging::log_execution_start(&args.function, parsed_args.as_deref());

    let mut executor = ContractExecutor::new(wasm_bytes.clone())?;
    executor.set_timeout(args.timeout);

    if let Some(storage) = initial_storage {
        executor.set_initial_storage(storage)?;
    }
    if !args.mock.is_empty() {
        executor.set_mock_specs(&args.mock)?;
    }

    let mut engine = DebuggerEngine::new(executor, args.breakpoint);

    if args.server {
        let token = args.token.clone().ok_or_else(|| anyhow::anyhow!("Token required for server mode"))?;
        let server = crate::server::debug_server::DebugServer::new(
            engine, 
            token, 
            args.tls_cert.as_deref(), 
            args.tls_key.as_deref()
        )?;
        
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(server.run(args.port))?;
        return Ok(());
    }

    if let Some(remote_addr) = args.remote {
        let token = args.token.clone().ok_or_else(|| anyhow::anyhow!("Token required for remote mode"))?;
        let rt = tokio::runtime::Runtime::new()?;
        // Use TLS if either cert or key is provided, or if we want to default to TLS if available.
        // For client-side, we might just want a flag --use-tls.
        // Let's assume for now if they provide ANY tls arg or if we want to detect it.
        let use_tls = args.tls_cert.is_some() || args.tls_key.is_some();
        let mut client = rt.block_on(crate::client::remote_client::RemoteClient::connect(&remote_addr, token, use_tls))?;
        
        println!("\nConnected to remote debugger.");
        let request = crate::protocol::DebugRequest::Execute {
            function: args.function.clone(),
            args: args.args.clone(),
        };
        
        let response = rt.block_on(client.send_request(request))?;
        println!("Remote Response: {:?}", response);
        return Ok(());
    }

    // Execute locally with debugging
    println!("\n--- Execution Start ---\n");
    if args.instruction_debug {
        print_info("Enabling instruction-level debugging...");
        engine.enable_instruction_debug(&wasm_bytes)?;

        if args.step_instructions {
            let step_mode = parse_step_mode(&args.step_mode);
            print_info(format!(
                "Starting instruction stepping in '{}' mode",
                args.step_mode
            ));
            engine.start_instruction_stepping(step_mode)?;
            run_instruction_stepping(&mut engine, &args.function, parsed_args.as_deref())?;
            return Ok(());
        }
    }

    print_info("\n--- Execution Start ---\n");
    output_writer.write("\n--- Execution Start ---\n")?;
    let storage_before = engine.executor().get_storage_snapshot()?;
    let result = engine.execute(&args.function, parsed_args.as_deref())?;
    let storage_after = engine.executor().get_storage_snapshot()?;
    print_success("\n--- Execution Complete ---\n");
    output_writer.write("\n--- Execution Complete ---\n")?;
    print_result(format!("Result: {:?}", result));
    output_writer.write(&format!("Result: {:?}", result))?;
    logging::log_execution_complete(&result);

    // Generate test if requested
    if let Some(test_path) = &args.generate_test {
        if let Some(record) = engine.executor().last_execution() {
            print_info(format!("\nGenerating unit test: {:?}", test_path));
            let test_code = crate::codegen::TestGenerator::generate(record, &args.contract)?;
            crate::codegen::TestGenerator::write_to_file(test_path, &test_code, args.overwrite)?;
            print_success(format!(
                "Unit test generated successfully at {:?}",
                test_path
            ));
        } else {
            print_warning("No execution record found to generate test.");
        }
    }

    let storage_diff = crate::inspector::storage::StorageInspector::compute_diff(
        &storage_before,
        &storage_after,
        &args.alert_on_change,
    );
    if !storage_diff.is_empty() || !args.alert_on_change.is_empty() {
        print_info("\n--- Storage Changes ---");
        crate::inspector::storage::StorageInspector::display_diff(&storage_diff);
    }

    if let Some(export_path) = &args.export_storage {
        print_info(format!("\nExporting storage to: {:?}", export_path));
        crate::inspector::storage::StorageState::export_to_file(&storage_after, export_path)?;
    }
    let mock_calls = engine.executor().get_mock_call_log();
    if !args.mock.is_empty() {
        display_mock_call_log(&mock_calls);
    }

    // Save budget info to history
    let host = engine.executor().host();
    let budget = crate::inspector::budget::BudgetInspector::get_cpu_usage(host);
    if let Ok(manager) = HistoryManager::new() {
        let record = RunHistory {
            date: chrono::Utc::now().to_rfc3339(),
            contract_hash: args.contract.to_string_lossy().to_string(),
            function: args.function.clone(),
            cpu_used: budget.cpu_instructions,
            memory_used: budget.memory_bytes,
        };
        let _ = manager.append_record(record);
    }
    let json_memory_summary = engine.executor().last_memory_summary().cloned();

    // Export storage if specified
    if let Some(export_path) = &args.export_storage {
        print_info(format!("Exporting storage to: {:?}", export_path));
        let storage_snapshot = engine.executor().get_storage_snapshot()?;
        crate::inspector::storage::StorageState::export_to_file(&storage_snapshot, export_path)?;
        print_success(format!(
            "Exported {} storage entries",
            storage_snapshot.len()
        ));
    }

    let mut json_events = None;
    if args.show_events {
        print_info("\n--- Events ---");
        let events = engine.executor().get_events()?;
        let filtered_events = if let Some(topic) = &args.filter_topic {
            crate::inspector::events::EventInspector::filter_events(&events, topic)
        } else {
            events
        };

        if filtered_events.is_empty() {
            print_warning("No events captured.");
        } else {
            for (i, event) in filtered_events.iter().enumerate() {
                print_info(format!("Event #{}:", i));
                if let Some(contract_id) = &event.contract_id {
                    logging::log_event_emitted(contract_id, event.topics.len());
                }
                print_info(format!(
                    "  Contract: {}",
                    event.contract_id.as_deref().unwrap_or("<none>")
                ));
                print_info(format!("  Topics: {:?}", event.topics));
                print_info(format!("  Data: {}", event.data));
            }
        }

        json_events = Some(filtered_events);
    }

    if !args.storage_filter.is_empty() {
        let storage_filter = crate::inspector::storage::StorageFilter::new(&args.storage_filter)
            .map_err(|e| DebuggerError::StorageError(format!("Invalid storage filter: {}", e)))?;

        print_info("\n--- Storage ---");
        let inspector = crate::inspector::StorageInspector::new();
        inspector.display_filtered(&storage_filter);
        print_info("(Storage view is currently placeholder data)");
    }

    let mut json_auth = None;
    if args.show_auth {
        let auth_tree = engine.executor().get_auth_tree()?;
        if args.json {
            // JSON mode: print the auth tree inline (will also be included in
            // the combined JSON object further below).
            let json_output = crate::inspector::auth::AuthInspector::to_json(&auth_tree)?;
            logging::log_display(json_output, logging::LogLevel::Info);
        } else {
            print_info("\n--- Authorization Tree ---");
            crate::inspector::auth::AuthInspector::display_with_summary(&auth_tree);
        }
        json_auth = Some(auth_tree);
    }

    let mut json_ledger = None;
    if args.show_ledger {
        print_info("\n--- Ledger Entries ---");
        let mut ledger_inspector = crate::inspector::ledger::LedgerEntryInspector::new();
        ledger_inspector.set_ttl_warning_threshold(args.ttl_warning_threshold);

        match engine.executor_mut().finish() {
            Ok((footprint, storage)) => {
                let mut footprint_map = std::collections::HashMap::new();
                for (k, v) in &footprint.0 {
                    footprint_map.insert(k.clone(), *v);
                }

                for (key, val_opt) in &storage.map {
                    if let Some(access_type) = footprint_map.get(key) {
                        if let Some((entry, ttl)) = val_opt {
                            let key_str = format!("{:?}", **key);
                            let storage_type =
                                if key_str.contains("Temporary") || key_str.contains("temporary") {
                                    crate::inspector::ledger::StorageType::Temporary
                                } else if key_str.contains("Instance")
                                    || key_str.contains("instance")
                                    || key_str.contains("LedgerKeyContractInstance")
                                {
                                    crate::inspector::ledger::StorageType::Instance
                                } else {
                                    crate::inspector::ledger::StorageType::Persistent
                                };

                            use soroban_env_host::storage::AccessType;
                            let is_read = true; // Everything in the footprint is at least read
                            let is_write = matches!(*access_type, AccessType::ReadWrite);

                            ledger_inspector.add_entry(
                                format!("{:?}", **key),
                                format!("{:?}", **entry),
                                storage_type,
                                ttl.unwrap_or(0),
                                is_read,
                                is_write,
                            );
                        }
                    }
                }
            }
            Err(e) => {
                print_warning(format!("Failed to extract ledger footprint: {}", e));
            }
        }

        ledger_inspector.display();
        ledger_inspector.display_warnings();
        json_ledger = Some(ledger_inspector);
    }

    if args.json
        || args
            .format
            .as_deref()
            .map(|f| f.eq_ignore_ascii_case("json"))
            .unwrap_or(false)
    {
        let mut output = serde_json::json!({
            "result": result,
            "sha256": wasm_hash,
            "alerts": storage_diff.triggered_alerts,
        });

        if let Some(events) = json_events {
            output["events"] = serde_json::Value::Array(
                events
                    .into_iter()
                    .map(|event| {
                        serde_json::json!({
                            "contract_id": event.contract_id,
                            "topics": event.topics,
                            "data": event.data,
                        })
                    })
                    .collect(),
            );
        }
        if let Some(auth_tree) = json_auth {
            output["auth"] = crate::inspector::auth::AuthInspector::to_json_value(&auth_tree);
        }
        if !mock_calls.is_empty() {
            output["mock_calls"] = serde_json::Value::Array(
                mock_calls
                    .iter()
                    .map(|entry| {
                        serde_json::json!({
                            "contract_id": entry.contract_id,
                            "function": entry.function,
                            "args_count": entry.args_count,
                            "mocked": entry.mocked,
                            "returned": entry.returned,
                        })
                    })
                    .collect(),
            );
        }
        if let Some(ref ledger) = json_ledger {
            output["ledger_entries"] = ledger.to_json();
        }
        if let Some(memory_summary) = json_memory_summary {
            output["memory_summary"] = serde_json::to_value(memory_summary).map_err(|e| {
                DebuggerError::FileError(format!("Failed to serialize memory summary: {}", e))
            })?;
        }

        let json_output = serde_json::to_string_pretty(&output).map_err(|e| {
            DebuggerError::FileError(format!("Failed to serialize output: {}", e))output["instruction_counts"] = serde_json::json!({
        })?;ap(|(name, count)| {
        logging::log_display(&json_output, logging::LogLevel::Info);
        output_writer.write(&json_output)?;                   "function": name,
    }                        "count": count,
rcentage": ((*count as f64 / instr_counts.total as f64) * 100.0)
    output_writer.flush()?;                    })

    // Display instruction count per function if available
    if let Some(instr_counts) = get_instruction_counts(&engine) {
        display_instruction_counts(&instr_counts);}
        
        // Include in JSON outputput = serde_json::to_string_pretty(&output).map_err(|e| {
        if args.jsonrError::FileError(format!("Failed to serialize output: {}", e))
            || args
                .formaty(&json_output, logging::LogLevel::Info);
                .as_deref()
                .map(|f| f.eq_ignore_ascii_case("json"))
                .unwrap_or(false)
        {
            // Already handled above, but ensure it's in output
        }/ Display instruction count per function if available
    }    if let Some(instr_counts) = get_instruction_counts(&engine) {

    // Show confirmation message if file was written
    if let Some(output_path) = &args.save_output {put
        print_success(format!(
            "\n✓ Output saved to: {}",
            output_path.display()     .format
        ));           .as_deref()
    }                .map(|f| f.eq_ignore_ascii_case("json"))
      .unwrap_or(false)
    Ok(())       {
}            // Already handled above, but ensure it's in output

/// Structure to hold instruction counts per function
#[derive(Debug, Clone, serde::Serialize)]
struct InstructionCounts { was written
    function_counts: Vec<(String, u64)>,(output_path) = &args.save_output {
    total: u64,       print_success(format!(
}            "\n✓ Output saved to: {}",

/// Get instruction counts from the debugger engine
fn get_instruction_counts(engine: &DebuggerEngine) -> Option<InstructionCounts> {
    // Try to get instruction counts from the executor
    if let Ok(counts) = engine.executor().get_instruction_counts() {
        Some(counts)
    } else {
        Nonetructure to hold instruction counts per function
    }[derive(Debug, Clone, serde::Serialize)]
}struct InstructionCounts {

/// Display instruction counts per function in a formatted table
fn display_instruction_counts(counts: &InstructionCounts) {
    if counts.function_counts.is_empty() {
        return;et instruction counts from the debugger engine
    }fn get_instruction_counts(engine: &DebuggerEngine) -> Option<InstructionCounts> {

    print_info("\n--- Instruction Count per Function ---");    if let Ok(counts) = engine.executor().get_instruction_counts() {

    // Calculate percentages
    let percentages: Vec<f64> = counts
        .function_counts
        .iter()
        .map(|(_, count)| {
            if counts.total > 0 {ble
                (*count as f64 / counts.total as f64) * 100.0on_counts(counts: &InstructionCounts) {
            } else {ion_counts.is_empty() {
                0.0n;
            }
        })
        .collect();    print_info("\n--- Instruction Count per Function ---");

    // Find max widths for alignment
    let max_func_width = counts<f64> = counts
        .function_countson_counts
        .iter()
        .map(|(name, _)| name.len())(_, count)| {
        .max()total > 0 {
        .unwrap_or(20)*count as f64 / counts.total as f64) * 100.0
        .max(10);
    let max_count_width = counts
        .function_counts
        .iter()
        .map(|(_, count)| count.to_string().len())ct();
        .max()
        .unwrap_or(10)idths for alignment
        .max(10);    let max_func_width = counts
ounts
    // Print header
    let header = format!(
        "{:<width1$} | {:>width2$} | {:>8$}",
        "Function",
        "Instructions",
        "Percentage",
        width1 = max_func_width,
        width2 = max_count_width,
        width3 = 10  .map(|(_, count)| count.to_string().len())
    );
    print_info(&header);
    print_info(&"-".repeat(header.len()));        .max(10);

    // Print rows
    for ((func_name, count), percentage) in counts.function_counts.iter().zip(percentages.iter()) {
        let row = format!(
            "{:<width1$} | {:>width2$} | {:>7.2$}%",
            func_name,ons",
            count,
            percentage,
            width1 = max_func_width,
            width2 = max_count_width,
            width3 = 8
        );
        print_info(&row);rint_info(&"-".repeat(header.len()));
    }

    print_info(&"-".repeat(header.len())); percentage) in counts.function_counts.iter().zip(percentages.iter()) {
    let total_row = format!(
        "{:<width1$} | {:>width2$}",width1$} | {:>width2$} | {:>7.2$}%",
        "TOTAL",,
        counts.total,
        width1 = max_func_width,
        width2 = max_count_width      width1 = max_func_width,
    );unt_width,
    print_info(&total_row);           width3 = 8
}        );

        print_info(&row);
    }

/// Parse JSON storage into a string for now (will be improved later)
fn parse_storage(json: &str) -> Result<String> {
    // Basic validation
    serde_json::from_str::<serde_json::Value>(json)
        .with_context(|| format!("Invalid JSON storage: {}", json))?;
    Ok(json.to_string())
}

/// Execute the upgrade-check command
pub fn upgrade_check(args: UpgradeCheckArgs) -> Result<()> {
    println!("Loading old contract: {:?}", args.old);
    let old_wasm = fs::read(&args.old)
        .with_context(|| format!("Failed to read old WASM file: {:?}", args.old))?;

    println!("Loading new contract: {:?}", args.new);
    let new_wasm = fs::read(&args.new)
        .with_context(|| format!("Failed to read new WASM file: {:?}", args.new))?;

    // Optionally run test inputs against both versions
    let execution_diffs = if let Some(inputs_json) = &args.test_inputs {
        run_test_inputs(inputs_json, &old_wasm, &new_wasm)?
    } else {
        Vec::new()
    };

    let old_path = args.old.to_string_lossy().to_string();
    let new_path = args.new.to_string_lossy().to_string();

    let report = UpgradeAnalyzer::analyze(&old_wasm, &new_wasm, &old_path, &new_path, execution_diffs)?;

    let output = match args.output.as_str() {
        "json" => serde_json::to_string_pretty(&report)?,
        _ => format_text_report(&report),
    };

    if let Some(out_file) = &args.output_file {
        fs::write(out_file, &output)
            .with_context(|| format!("Failed to write report to {:?}", out_file))?;
        println!("Report written to {:?}", out_file);
    } else {
        println!("{}", output);
    }

    if !report.is_compatible {
        anyhow::bail!("Contracts are not compatible: {} breaking change(s) detected", report.breaking_changes.len());
    }

    Ok(())
}

/// Run test inputs against both WASM versions and collect diffs
fn run_test_inputs(
    inputs_json: &str,
    old_wasm: &[u8],
    new_wasm: &[u8],
) -> Result<Vec<ExecutionDiff>> {
    let inputs: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(inputs_json).with_context(|| {
            "Invalid --test-inputs JSON: expected an object mapping function names to arg arrays"
        })?;

    let mut diffs = Vec::new();

    for (func_name, args_val) in &inputs {
        let args_str = args_val.to_string();

        let old_result = invoke_wasm(old_wasm, func_name, &args_str);
        let new_result = invoke_wasm(new_wasm, func_name, &args_str);

        let outputs_match = old_result == new_result;
        diffs.push(ExecutionDiff {
            function: func_name.clone(),
            args: args_str,
            old_result,
            new_result,
            outputs_match,
        });
    }

    Ok(diffs)
}

/// Invoke a function on a WASM contract and return a string representation of the result
fn invoke_wasm(wasm: &[u8], function: &str, args: &str) -> String {
    match ContractExecutor::new(wasm.to_vec()) {
        Err(e) => format!("Err(executor: {})", e),
        Ok(executor) => {
            let mut engine = DebuggerEngine::new(executor, vec![]);
            let parsed = if args == "null" || args == "[]" {
                None
            } else {
                Some(args.to_string())
            };
            match engine.execute(function, parsed.as_deref()) {
                Ok(val) => format!("Ok({:?})", val),
                Err(e) => format!("Err({})", e),
            }
        }
    }
}

/// Format a compatibility report as human-readable text
fn format_text_report(report: &CompatibilityReport) -> String {
    let mut out = String::new();

    out.push_str("Contract Upgrade Compatibility Report\n");
    out.push_str("======================================\n");
    out.push_str(&format!("Old: {}\n", report.old_wasm_path));
    out.push_str(&format!("New: {}\n", report.new_wasm_path));
    out.push('\n');

    let status = if report.is_compatible { "COMPATIBLE" } else { "INCOMPATIBLE" };
    out.push_str(&format!("Status: {}\n", status));

    out.push('\n');
    out.push_str(&format!("Breaking Changes ({}):\n", report.breaking_changes.len()));
    if report.breaking_changes.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for change in &report.breaking_changes {
            out.push_str(&format!("  {}\n", change));
        }
    }

    out.push('\n');
    out.push_str(&format!("Non-Breaking Changes ({}):\n", report.non_breaking_changes.len()));
    if report.non_breaking_changes.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for change in &report.non_breaking_changes {
            out.push_str(&format!("  {}\n", change));
        }
    }

    if !report.execution_diffs.is_empty() {
        out.push('\n');
        out.push_str(&format!("Execution Diffs ({}):\n", report.execution_diffs.len()));
        for diff in &report.execution_diffs {
            let match_str = if diff.outputs_match { "MATCH" } else { "MISMATCH" };
            out.push_str(&format!(
                "  {} args={} OLD={} NEW={} [{}]\n",
                diff.function, diff.args, diff.old_result, diff.new_result, match_str
            ));
        }
    }

    out.push('\n');
    let old_names: Vec<&str> = report.old_functions.iter().map(|f| f.name.as_str()).collect();
    let new_names: Vec<&str> = report.new_functions.iter().map(|f| f.name.as_str()).collect();
    out.push_str(&format!("Old Functions ({}): {}\n", old_names.len(), old_names.join(", ")));
    out.push_str(&format!("New Functions ({}): {}\n", new_names.len(), new_names.join(", ")));

    out
}
    print_info(&"-".repeat(header.len()));
    let total_row = format!(
        "{:<width1$} | {:>width2$}",
        "TOTAL",
        counts.total,
        width1 = max_func_width,
        width2 = max_count_width
    );
    print_info(&total_row);
}
