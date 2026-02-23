/// Interactive REPL mode for contract exploration
///
/// This module provides a Read-Eval-Print Loop (REPL) interface for
/// interactively calling contract functions, inspecting storage, and
/// exploring contract state without restarting.
pub mod commands;
pub mod executor;
pub mod session;

pub use session::ReplSession;

use crate::Result;
use std::path::PathBuf;

/// Configuration for starting the REPL
#[derive(Debug, Clone)]
pub struct ReplConfig {
    pub contract_path: PathBuf,
    pub network_snapshot: Option<PathBuf>,
    pub storage: Option<String>,
}

/// Start the REPL interactive session
pub async fn start_repl(config: ReplConfig) -> Result<()> {
    let mut session = ReplSession::new(config)?;
    session.run().await?;
    Ok(())
}
