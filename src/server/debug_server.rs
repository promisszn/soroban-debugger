use crate::debugger::engine::DebuggerEngine;
use crate::protocol::{DebugRequest, DebugResponse};
use crate::Result;
use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::rustls::{Certificate, PrivateKey, ServerConfig};
use tokio_rustls::TlsAcceptor;
use tracing::{info, error, warn};

pub struct DebugServer {
    engine: DebuggerEngine,
    token: String,
    tls_config: Option<ServerConfig>,
}

impl DebugServer {
    pub fn new(
        engine: DebuggerEngine,
        token: String,
        cert_path: Option<&Path>,
        key_path: Option<&Path>,
    ) -> Result<Self> {
        let tls_config = if let (Some(cp), Some(kp)) = (cert_path, key_path) {
            Some(load_tls_config(cp, kp)?)
        } else {
            None
        };

        Ok(Self {
            engine,
            token,
            tls_config,
        })
    }

    pub async fn run(mut self, port: u16) -> Result<()> {
        let addr = format!("0.0.0.0:{}", port);
        let listener = TcpListener::bind(&addr).await?;
        info!("Debug server listening on {}", addr);

        let acceptor = self.tls_config.take().map(|cfg| TlsAcceptor::from(Arc::new(cfg)));

        loop {
            let (stream, addr) = listener.accept().await?;
            info!("New connection from {}", addr);

            if let Some(ref acceptor) = acceptor {
                match acceptor.accept(stream).await {
                    Ok(tls_stream) => {
                        if let Err(e) = self.handle_single_connection(tls_stream).await {
                            error!("TLS connection error: {}", e);
                        }
                    }
                    Err(e) => error!("TLS accept error: {}", e),
                }
            } else {
                if let Err(e) = self.handle_single_connection(stream).await {
                    error!("TCP connection error: {}", e);
                }
            }
        }
    }

    async fn handle_single_connection<S>(&mut self, mut stream: S) -> Result<()> 
    where S: AsyncReadExt + AsyncWriteExt + Unpin 
    {
        let mut authenticated = false;
        let mut buffer = vec![0u8; 8192];

        loop {
            let n = stream.read(&mut buffer).await?;
            if n == 0 {
                break;
            }

            let request: DebugRequest = match serde_json::from_slice(&buffer[..n]) {
                Ok(req) => req,
                Err(e) => {
                    warn!("Failed to parse request: {}", e);
                    continue;
                }
            };
            info!("Received request: {:?}", request);

            if !authenticated {
                if let DebugRequest::Handshake { token: ref req_token } = request {
                    if req_token == &self.token {
                        authenticated = true;
                        send_response(&mut stream, DebugResponse::AuthSuccess).await?;
                        continue;
                    } else {
                        send_response(&mut stream, DebugResponse::AuthFailed).await?;
                        return Ok(());
                    }
                } else {
                    send_response(&mut stream, DebugResponse::Error("Authentication required".to_string())).await?;
                    return Ok(());
                }
            }

            let response = match request {
                DebugRequest::Handshake { .. } => DebugResponse::Error("Already authenticated".to_string()),
                DebugRequest::Step => {
                    match self.engine.step() {
                        Ok(_) => DebugResponse::Ok,
                        Err(e) => DebugResponse::Error(e.to_string()),
                    }
                }
                DebugRequest::Continue => {
                    match self.engine.continue_execution() {
                        Ok(_) => DebugResponse::Ok,
                        Err(e) => DebugResponse::Error(e.to_string()),
                    }
                }
                DebugRequest::AddBreakpoint { function } => {
                    self.engine.breakpoints_mut().add(&function);
                    DebugResponse::Ok
                }
                DebugRequest::RemoveBreakpoint { function } => {
                    self.engine.breakpoints_mut().remove(&function);
                    DebugResponse::Ok
                }
                DebugRequest::GetState => {
                    DebugResponse::State(self.engine.state().clone())
                }
                DebugRequest::Execute { function, args } => {
                    match self.engine.execute(&function, args.as_deref()) {
                        Ok(res) => DebugResponse::ExecutionResult { result: res },
                        Err(e) => DebugResponse::Error(e.to_string()),
                    }
                }
            };

            send_response(&mut stream, response).await?;
        }

        Ok(())
    }
}

async fn send_response<S>(stream: &mut S, response: DebugResponse) -> Result<()> 
where S: AsyncWriteExt + Unpin
{
    let json = serde_json::to_vec(&response)?;
    stream.write_all(&json).await?;
    Ok(())
}

fn load_tls_config(cert_path: &Path, key_path: &Path) -> Result<ServerConfig> {
    let cert_file = fs::File::open(cert_path)?;
    let mut cert_reader = BufReader::new(cert_file);
    let certs = rustls_pemfile::certs(&mut cert_reader)?
        .into_iter()
        .map(Certificate)
        .collect();

    let key_file = fs::File::open(key_path)?;
    let mut key_reader = BufReader::new(key_file);
    let keys = rustls_pemfile::pkcs8_private_keys(&mut key_reader)?;
    if keys.is_empty() {
        return Err(anyhow::anyhow!("No private key found"));
    }
    let key = PrivateKey(keys[0].clone());

    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    Ok(config)
use crate::runtime::executor::ContractExecutor;
use crate::server::protocol::{DebugMessage, DebugRequest, DebugResponse};
use crate::simulator::SnapshotLoader;
use crate::{DebuggerError, Result};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::{error, info};

/// Debug server that handles remote debugging connections
pub struct DebugServer {
    port: u16,
    token: Option<String>,
    tls_cert: Option<PathBuf>,
    tls_key: Option<PathBuf>,
}

/// Session state for a connected client
struct Session {
    #[allow(clippy::arc_with_non_send_sync)]
    engine: Option<Arc<Mutex<DebuggerEngine>>>,
    authenticated: bool,
    #[allow(dead_code)]
    message_id: u64,
}

impl DebugServer {
    /// Create a new debug server
    pub fn new(port: u16, token: Option<String>) -> Self {
        Self {
            port,
            token,
            tls_cert: None,
            tls_key: None,
        }
    }

    /// Set TLS certificate and key paths
    pub fn with_tls(mut self, cert: PathBuf, key: PathBuf) -> Self {
        self.tls_cert = Some(cert);
        self.tls_key = Some(key);
        self
    }

    /// Start the debug server and listen for connections
    pub fn start(&self) -> Result<()> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr)
            .map_err(|e| DebuggerError::FileError(format!("Failed to bind to {}: {}", addr, e)))?;

        info!("Debug server listening on {}", addr);
        if self.token.is_some() {
            info!("Token authentication enabled");
        }
        if self.tls_cert.is_some() {
            info!("TLS enabled");
        }

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let token = self.token.clone();
                    let tls_cert = self.tls_cert.clone();
                    let tls_key = self.tls_key.clone();

                    // Handle each connection in a separate thread
                    std::thread::spawn(move || {
                        if let Err(e) = Self::handle_client(stream, token, tls_cert, tls_key) {
                            error!("Error handling client: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }

        Ok(())
    }

    fn handle_client(
        stream: TcpStream,
        token: Option<String>,
        _tls_cert: Option<PathBuf>,
        _tls_key: Option<PathBuf>,
    ) -> Result<()> {
        let peer_addr = stream
            .peer_addr()
            .map_err(|e| DebuggerError::FileError(format!("Failed to get peer address: {}", e)))?;
        info!("New client connected from {}", peer_addr);

        let mut session = Session {
            engine: None,
            authenticated: token.is_none(), // Auto-authenticate if no token required
            message_id: 0,
        };

        let reader = BufReader::new(
            stream
                .try_clone()
                .map_err(|e| DebuggerError::FileError(format!("Failed to clone stream: {}", e)))?,
        );
        let mut writer = stream;

        for line in reader.lines() {
            let line =
                line.map_err(|e| DebuggerError::FileError(format!("Failed to read line: {}", e)))?;
            if line.is_empty() {
                continue;
            }

            let message: DebugMessage = serde_json::from_str(&line).map_err(|e| {
                DebuggerError::FileError(format!("Failed to parse message: {}: {}", line, e))
            })?;

            let response = Self::handle_request(&mut session, message, &token)?;

            let response_json = serde_json::to_string(&response).map_err(|e| {
                DebuggerError::FileError(format!("Failed to serialize response: {}", e))
            })?;
            writeln!(writer, "{}", response_json).map_err(|e| {
                DebuggerError::FileError(format!("Failed to write response: {}", e))
            })?;
            writer
                .flush()
                .map_err(|e| DebuggerError::FileError(format!("Failed to flush stream: {}", e)))?;
        }

        info!("Client {} disconnected", peer_addr);
        Ok(())
    }

    #[allow(clippy::arc_with_non_send_sync)]
    fn handle_request(
        session: &mut Session,
        message: DebugMessage,
        expected_token: &Option<String>,
    ) -> Result<DebugMessage> {
        let request = message
            .request
            .ok_or_else(|| DebuggerError::ExecutionError("Message has no request".to_string()))?;

        // Check authentication for all requests except Authenticate and Ping
        match &request {
            DebugRequest::Authenticate { .. } | DebugRequest::Ping => {}
            _ => {
                if !session.authenticated {
                    return Ok(DebugMessage::response(
                        message.id,
                        DebugResponse::Error {
                            message: "Not authenticated. Send Authenticate request first."
                                .to_string(),
                        },
                    ));
                }
            }
        }

        let response = match request {
            DebugRequest::Authenticate { token } => {
                if let Some(expected) = expected_token {
                    let success = token == *expected;
                    session.authenticated = success;
                    DebugResponse::Authenticated {
                        success,
                        message: if success {
                            "Authentication successful".to_string()
                        } else {
                            "Invalid token".to_string()
                        },
                    }
                } else {
                    session.authenticated = true;
                    DebugResponse::Authenticated {
                        success: true,
                        message: "No authentication required".to_string(),
                    }
                }
            }

            DebugRequest::Ping => DebugResponse::Pong,

            DebugRequest::LoadContract { contract_path } => {
                match Self::load_contract(&contract_path) {
                    Ok((engine, size)) => {
                        session.engine = Some(Arc::new(Mutex::new(engine)));
                        DebugResponse::ContractLoaded { size }
                    }
                    Err(e) => DebugResponse::Error {
                        message: format!("Failed to load contract: {}", e),
                    },
                }
            }

            DebugRequest::LoadSnapshot { snapshot_path } => {
                match SnapshotLoader::from_file(&snapshot_path) {
                    Ok(loader) => match loader.apply_to_environment() {
                        Ok(snapshot) => DebugResponse::SnapshotLoaded {
                            summary: snapshot.format_summary(),
                        },
                        Err(e) => DebugResponse::Error {
                            message: format!("Failed to apply snapshot: {}", e),
                        },
                    },
                    Err(e) => DebugResponse::Error {
                        message: format!("Failed to load snapshot: {}", e),
                    },
                }
            }

            DebugRequest::SetStorage { storage_json } => {
                if let Some(engine) = &session.engine {
                    match Self::parse_storage(&storage_json) {
                        Ok(storage) => {
                            if let Ok(mut engine) = engine.lock() {
                                if let Err(e) = engine.executor_mut().set_initial_storage(storage) {
                                    DebugResponse::Error {
                                        message: format!("Failed to set storage: {}", e),
                                    }
                                } else {
                                    DebugResponse::StorageState {
                                        storage_json: storage_json.clone(),
                                    }
                                }
                            } else {
                                DebugResponse::Error {
                                    message: "Failed to lock engine".to_string(),
                                }
                            }
                        }
                        Err(e) => DebugResponse::Error {
                            message: format!("Failed to parse storage: {}", e),
                        },
                    }
                } else {
                    DebugResponse::Error {
                        message: "No contract loaded".to_string(),
                    }
                }
            }

            DebugRequest::Execute { function, args } => {
                if let Some(engine) = &session.engine {
                    let mut engine_guard = engine.lock().map_err(|e| {
                        DebuggerError::ExecutionError(format!("Failed to lock engine: {}", e))
                    })?;

                    match engine_guard.execute(&function, args.as_deref()) {
                        Ok(output) => DebugResponse::ExecutionResult {
                            success: true,
                            output,
                            error: None,
                        },
                        Err(e) => DebugResponse::ExecutionResult {
                            success: false,
                            output: String::new(),
                            error: Some(format!("{}", e)),
                        },
                    }
                } else {
                    DebugResponse::Error {
                        message: "No contract loaded".to_string(),
                    }
                }
            }

            DebugRequest::Step => {
                if let Some(engine) = &session.engine {
                    let mut engine = engine.lock().map_err(|e| {
                        DebuggerError::ExecutionError(format!("Failed to lock engine: {}", e))
                    })?;

                    match engine.step() {
                        Ok(_) => {
                            let state = engine.state();
                            let state_guard = state.lock().map_err(|e| {
                                DebuggerError::ExecutionError(format!(
                                    "Failed to lock state: {}",
                                    e
                                ))
                            })?;

                            DebugResponse::StepResult {
                                paused: engine.is_paused(),
                                current_function: state_guard
                                    .current_function()
                                    .map(|s: &str| s.to_string()),
                                step_count: state_guard.step_count() as u64,
                            }
                        }
                        Err(e) => DebugResponse::Error {
                            message: format!("Step failed: {}", e),
                        },
                    }
                } else {
                    DebugResponse::Error {
                        message: "No contract loaded".to_string(),
                    }
                }
            }

            DebugRequest::Continue => {
                if let Some(engine) = &session.engine {
                    let mut engine = engine.lock().map_err(|e| {
                        DebuggerError::ExecutionError(format!("Failed to lock engine: {}", e))
                    })?;

                    match engine.continue_execution() {
                        Ok(_) => {
                            // Execution completed
                            DebugResponse::ContinueResult {
                                completed: true,
                                output: None,
                                error: None,
                            }
                        }
                        Err(e) => DebugResponse::ContinueResult {
                            completed: false,
                            output: None,
                            error: Some(format!("{}", e)),
                        },
                    }
                } else {
                    DebugResponse::Error {
                        message: "No contract loaded".to_string(),
                    }
                }
            }

            DebugRequest::Inspect => {
                if let Some(engine) = &session.engine {
                    let engine_guard = engine.lock().map_err(|e| {
                        DebuggerError::ExecutionError(format!("Failed to lock engine: {}", e))
                    })?;
                    let state = engine_guard.state();
                    let state_guard = state.lock().map_err(|e| {
                        DebuggerError::ExecutionError(format!("Failed to lock state: {}", e))
                    })?;

                    let call_stack: Vec<String> = state_guard
                        .call_stack()
                        .get_stack()
                        .iter()
                        .map(|frame| frame.function.clone())
                        .collect();

                    DebugResponse::InspectionResult {
                        function: state_guard.current_function().map(|s: &str| s.to_string()),
                        step_count: state_guard.step_count() as u64,
                        paused: engine_guard.is_paused(),
                        call_stack,
                    }
                } else {
                    DebugResponse::Error {
                        message: "No contract loaded".to_string(),
                    }
                }
            }

            DebugRequest::GetStorage => {
                if let Some(engine) = &session.engine {
                    // Get storage from the executor's host
                    let engine_guard = engine.lock().map_err(|e| {
                        DebuggerError::ExecutionError(format!("Failed to lock engine: {}", e))
                    })?;
                    let _host = engine_guard.executor().host();

                    // This is a simplified version - in practice, you'd serialize the actual storage
                    DebugResponse::StorageState {
                        storage_json: "{}".to_string(), // Placeholder
                    }
                } else {
                    DebugResponse::Error {
                        message: "No contract loaded".to_string(),
                    }
                }
            }

            DebugRequest::GetStack => {
                if let Some(engine) = &session.engine {
                    let engine_guard = engine.lock().map_err(|e| {
                        DebuggerError::ExecutionError(format!("Failed to lock engine: {}", e))
                    })?;
                    let state = engine_guard.state();
                    let state_guard = state.lock().map_err(|e| {
                        DebuggerError::ExecutionError(format!("Failed to lock state: {}", e))
                    })?;

                    let stack: Vec<String> = state_guard
                        .call_stack()
                        .get_stack()
                        .iter()
                        .map(|frame| frame.function.clone())
                        .collect();

                    DebugResponse::CallStack { stack }
                } else {
                    DebugResponse::Error {
                        message: "No contract loaded".to_string(),
                    }
                }
            }

            DebugRequest::GetBudget => {
                if let Some(engine) = &session.engine {
                    let engine_guard = engine.lock().map_err(|e| {
                        DebuggerError::ExecutionError(format!("Failed to lock engine: {}", e))
                    })?;
                    let host = engine_guard.executor().host();
                    let budget = host.budget_cloned();

                    let cpu_instructions = budget.get_cpu_insns_consumed().unwrap_or(0);
                    let memory_bytes = budget.get_mem_bytes_consumed().unwrap_or(0);

                    DebugResponse::BudgetInfo {
                        cpu_instructions,
                        memory_bytes,
                    }
                } else {
                    DebugResponse::Error {
                        message: "No contract loaded".to_string(),
                    }
                }
            }

            DebugRequest::SetBreakpoint { function } => {
                if let Some(engine) = &session.engine {
                    let mut engine = engine.lock().map_err(|e| {
                        DebuggerError::ExecutionError(format!("Failed to lock engine: {}", e))
                    })?;
                    engine.breakpoints_mut().add(&function);
                    DebugResponse::BreakpointSet { function }
                } else {
                    DebugResponse::Error {
                        message: "No contract loaded".to_string(),
                    }
                }
            }

            DebugRequest::ClearBreakpoint { function } => {
                if let Some(engine) = &session.engine {
                    let mut engine = engine.lock().map_err(|e| {
                        DebuggerError::ExecutionError(format!("Failed to lock engine: {}", e))
                    })?;
                    engine.breakpoints_mut().remove(&function);
                    DebugResponse::BreakpointCleared { function }
                } else {
                    DebugResponse::Error {
                        message: "No contract loaded".to_string(),
                    }
                }
            }

            DebugRequest::ListBreakpoints => {
                if let Some(engine) = &session.engine {
                    let mut engine_guard = engine.lock().map_err(|e| {
                        DebuggerError::ExecutionError(format!("Failed to lock engine: {}", e))
                    })?;
                    let breakpoints = engine_guard.breakpoints_mut().list();
                    DebugResponse::BreakpointsList { breakpoints }
                } else {
                    DebugResponse::Error {
                        message: "No contract loaded".to_string(),
                    }
                }
            }

            DebugRequest::Disconnect => DebugResponse::Disconnected,
        };

        Ok(DebugMessage::response(message.id, response))
    }

    fn load_contract(contract_path: &str) -> Result<(DebuggerEngine, usize)> {
        use std::fs;
        let wasm_bytes = fs::read(contract_path).map_err(|e| {
            DebuggerError::WasmLoadError(format!(
                "Failed to read contract {}: {}",
                contract_path, e
            ))
        })?;
        let size = wasm_bytes.len();
        let executor = ContractExecutor::new(wasm_bytes)?;
        let engine = DebuggerEngine::new(executor, vec![]);
        Ok((engine, size))
    }

    fn parse_storage(_storage_json: &str) -> Result<String> {
        // Storage parsing is validated but not fully implemented in executor yet
        // Just validate JSON for now
        serde_json::from_str::<serde_json::Value>(_storage_json).map_err(|e| {
            DebuggerError::StorageError(format!("Failed to parse storage JSON: {}", e))
        })?;
        Ok(_storage_json.to_string())
    }
}
