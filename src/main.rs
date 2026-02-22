use clap::{CommandFactory, Parser};
use clap_complete::generate;
use soroban_debugger::cli::{Cli, Commands, Verbosity};
use soroban_debugger::ui::formatter::Formatter;
use std::io;

fn initialize_tracing(verbosity: Verbosity) {
    let log_level = verbosity.to_log_level();
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("soroban_debugger={}", log_level).into());

    let use_json = std::env::var("SOROBAN_DEBUG_JSON").is_ok();

    let subscriber = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(true)
        .with_level(true)
        .with_env_filter(env_filter);

    if use_json {
        subscriber.json().init();
    } else {
        subscriber.init();
    }
}

fn handle_deprecations(cli: &mut Cli) {
    match &mut cli.command {
        Some(Commands::Run(args)) => {
            if let Some(wasm) = args.wasm.take() {
                tracing::warn!("{}", Formatter::warning("Warning: --wasm and --contract-path are deprecated. Please use --contract instead."));
                args.contract = wasm;
            }
            if let Some(snapshot) = args.snapshot.take() {
                tracing::warn!(
                    "{}",
                    Formatter::warning(
                        "Warning: --snapshot is deprecated. Please use --network-snapshot instead."
                    )
                );
                args.network_snapshot = Some(snapshot);
            }
        }
        Some(Commands::Interactive(args)) => {
            if let Some(wasm) = args.wasm.take() {
                tracing::warn!("{}", Formatter::warning("Warning: --wasm and --contract-path are deprecated. Please use --contract instead."));
                args.contract = wasm;
            }
            if let Some(snapshot) = args.snapshot.take() {
                tracing::warn!(
                    "{}",
                    Formatter::warning(
                        "Warning: --snapshot is deprecated. Please use --network-snapshot instead."
                    )
                );
                args.network_snapshot = Some(snapshot);
            }
        }
        Some(Commands::Inspect(args)) => {
            if let Some(wasm) = args.wasm.take() {
                tracing::warn!("{}", Formatter::warning("Warning: --wasm and --contract-path are deprecated. Please use --contract instead."));
                args.contract = wasm;
            }
        }
        Some(Commands::Optimize(args)) => {
            if let Some(wasm) = args.wasm.take() {
                tracing::warn!("{}", Formatter::warning("Warning: --wasm and --contract-path are deprecated. Please use --contract instead."));
                args.contract = wasm;
            }
            if let Some(snapshot) = args.snapshot.take() {
                tracing::warn!(
                    "{}",
                    Formatter::warning(
                        "Warning: --snapshot is deprecated. Please use --network-snapshot instead."
                    )
                );
                args.network_snapshot = Some(snapshot);
            }
        }
        Some(Commands::Profile(args)) => {
            if let Some(wasm) = args.wasm.take() {
                tracing::warn!("{}", Formatter::warning("Warning: --wasm and --contract-path are deprecated. Please use --contract instead."));
                args.contract = wasm;
            }
        }
        Some(Commands::Repl(args)) => {
            if let Some(wasm) = args.wasm.take() {
                tracing::warn!("{}", Formatter::warning("Warning: --wasm and --contract-path are deprecated. Please use --contract instead."));
                args.contract = wasm;
            }
            if let Some(snapshot) = args.snapshot.take() {
                tracing::warn!(
                    "{}",
                    Formatter::warning(
                        "Warning: --snapshot is deprecated. Please use --network-snapshot instead."
                    )
                );
                args.network_snapshot = Some(snapshot);
            }
        }
        _ => {}
    }
}

fn main() -> miette::Result<()> {
    Formatter::configure_colors_from_env();

    let mut cli = Cli::parse();
    handle_deprecations(&mut cli);
    let verbosity = cli.verbosity();

    initialize_tracing(verbosity);

    let config = soroban_debugger::config::Config::load_or_default();

    let result = match cli.command {
        Some(Commands::Run(mut args)) => {
            args.merge_config(&config);
            soroban_debugger::cli::commands::run(args, verbosity)
        }
        Some(Commands::Interactive(mut args)) => {
            args.merge_config(&config);
            soroban_debugger::cli::commands::interactive(args, verbosity)
        }
        Some(Commands::Tui(args)) => soroban_debugger::cli::commands::tui(args, verbosity),
        Some(Commands::Inspect(args)) => soroban_debugger::cli::commands::inspect(args, verbosity),
        Some(Commands::Optimize(args)) => {
            soroban_debugger::cli::commands::optimize(args, verbosity)
        }
        Some(Commands::UpgradeCheck(args)) => {
            soroban_debugger::cli::commands::upgrade_check(args, verbosity)
        }
        Some(Commands::Compare(args)) => soroban_debugger::cli::commands::compare(args),
        Some(Commands::Replay(args)) => soroban_debugger::cli::commands::replay(args, verbosity),
        Some(Commands::Completions(args)) => {
            let mut cmd = Cli::command();
            generate(args.shell, &mut cmd, "soroban-debug", &mut io::stdout());
            Ok(())
        }
        Some(Commands::Profile(args)) => soroban_debugger::cli::commands::profile(args),
        Some(Commands::Symbolic(args)) => {
            soroban_debugger::cli::commands::symbolic(args, verbosity)
        }
        Some(Commands::Server(args)) => soroban_debugger::cli::commands::server(args),
        Some(Commands::Remote(args)) => soroban_debugger::cli::commands::remote(args, verbosity),
        Some(Commands::Analyze(args)) => soroban_debugger::cli::commands::analyze(args, verbosity),
        Some(Commands::Repl(mut args)) => {
            args.merge_config(&config);
            tokio::runtime::Runtime::new()
                .map_err(|e| miette::miette!(e))?
                .block_on(soroban_debugger::cli::commands::repl(args))
                .map_err(|e| miette::miette!(e))
        }
        None => {
            if let Some(path) = cli.list_functions {
                return soroban_debugger::cli::commands::inspect(
                    soroban_debugger::cli::args::InspectArgs {
                        contract: path,
                        wasm: None,
                        functions: true,
                        metadata: false,
                        expected_hash: None,
                        dependency_graph: false,
                    },
                    verbosity,
                );
            }
            if cli.budget_trend {
                soroban_debugger::cli::commands::show_budget_trend(
                    cli.trend_contract.as_deref(),
                    cli.trend_function.as_deref(),
                )
            } else {
                let mut cmd = Cli::command();
                cmd.print_help().map_err(|e| miette::miette!(e))?;
                tracing::info!("");
                Ok(())
            }
        }
    };

    if let Err(err) = result {
        tracing::error!(
            "{}",
            Formatter::error(format!("Error handling deprecations: {err:#}"))
        );
        return Err(err);
    }

    Ok(())
}
