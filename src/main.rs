use anyhow::Result;
use clap::Parser;
use is_terminal::IsTerminal;
use soroban_debugger::cli::{Cli, Commands, Verbosity};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn show_banner() {
    let version = env!("CARGO_PKG_VERSION");
    println!("╔═══════════════════════════════════════╗");
    println!("║   SOROBAN DEBUGGER v{:<16}  ║", version);
    println!("║   Smart Contract Debugging Tool      ║");
    println!("╚═══════════════════════════════════════╝");
    println!();
}

fn initialize_tracing(verbosity: Verbosity) {
    let log_level = verbosity.to_log_level();
    let env_filter =
        std::env::var("RUST_LOG").unwrap_or_else(|_| format!("soroban_debugger={}", log_level));

    let use_json = std::env::var("SOROBAN_DEBUG_JSON").is_ok();

    if use_json {
        let json_layer = tracing_subscriber::fmt::layer()
            .json()
            .with_writer(std::io::stderr)
            .with_target(true)
            .with_level(true);

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| env_filter.into()),
            )
            .with(json_layer)
            .init();
    } else {
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_writer(std::io::stderr)
            .with_target(true)
            .with_level(true);

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| env_filter.into()),
            )
            .with(fmt_layer)
            .init();
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let verbosity = cli.verbosity();

    // Show ASCII banner if conditions are met
    let should_show_banner = std::io::stdout().is_terminal()
        && !cli.no_banner
        && std::env::var("NO_BANNER").is_err();
    
    if should_show_banner {
        show_banner();
    }

    initialize_tracing(verbosity);

    let result = match cli.command {
        Commands::Run(args) => soroban_debugger::cli::commands::run(args, verbosity),
        Commands::Interactive(args) => {
            soroban_debugger::cli::commands::interactive(args, verbosity)
        }
        Commands::Inspect(args) => soroban_debugger::cli::commands::inspect(args, verbosity),
        Commands::Optimize(args) => soroban_debugger::cli::commands::optimize(args, verbosity),
        Commands::UpgradeCheck(args) => {
            soroban_debugger::cli::commands::upgrade_check(args, verbosity)
        }
        Commands::Completions(_args) => {
            eprintln!("Completions command not yet implemented");
            return Ok(());
        }
        Commands::Compare(args) => soroban_debugger::cli::commands::compare(args),
    };

    if let Err(err) = result {
        eprintln!("Error: {err:#}");
        return Err(err);
    }

    Ok(())
}
