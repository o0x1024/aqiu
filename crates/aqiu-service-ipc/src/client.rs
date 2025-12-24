//! IPC Client implementation
//! 
//! Provides async client for connecting to aqiu-service daemon.

use crate::{IpcError, IpcRequest, IpcResponse, IpcResult, FrameHeader, IPC_PATH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{timeout, Duration};

#[cfg(unix)]
use tokio::net::UnixStream;

/// Default timeout for IPC operations
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// IPC Client configuration
#[derive(Debug, Clone)]
pub struct IpcConfig {
    /// Timeout for operations
    pub timeout: Duration,
    /// Number of retries
    pub max_retries: u32,
    /// Delay between retries
    pub retry_delay: Duration,
}

impl Default for IpcConfig {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            max_retries: 3,
            retry_delay: Duration::from_millis(200),
        }
    }
}

/// Connect to the service and return a stream
#[cfg(unix)]
pub async fn connect() -> IpcResult<UnixStream> {
    let path = std::path::Path::new(IPC_PATH);
    if !path.exists() {
        return Err(IpcError::ServiceUnavailable);
    }
    
    UnixStream::connect(IPC_PATH)
        .await
        .map_err(|e| IpcError::ConnectionFailed(e.to_string()))
}

/// Send a request and receive a response
#[cfg(unix)]
pub async fn send_request(request: &IpcRequest) -> IpcResult<IpcResponse> {
    send_request_with_config(request, &IpcConfig::default()).await
}

/// Send a request with custom config
#[cfg(unix)]
pub async fn send_request_with_config(
    request: &IpcRequest,
    config: &IpcConfig,
) -> IpcResult<IpcResponse> {
    let mut last_error = None;
    
    for attempt in 0..=config.max_retries {
        if attempt > 0 {
            tokio::time::sleep(config.retry_delay).await;
        }
        
        match timeout(config.timeout, send_request_inner(request)).await {
            Ok(Ok(response)) => return Ok(response),
            Ok(Err(e)) => {
                tracing::warn!("IPC request attempt {} failed: {}", attempt + 1, e);
                last_error = Some(e);
            }
            Err(_) => {
                tracing::warn!("IPC request attempt {} timed out", attempt + 1);
                last_error = Some(IpcError::Timeout);
            }
        }
    }
    
    Err(last_error.unwrap_or(IpcError::ServiceUnavailable))
}

#[cfg(unix)]
async fn send_request_inner(request: &IpcRequest) -> IpcResult<IpcResponse> {
    let mut stream = connect().await?;
    
    // Serialize request
    let payload = serde_json::to_vec(request)?;
    let header = FrameHeader::new(payload.len() as u32);
    
    // Write header + payload
    stream.write_all(&header.to_bytes()).await?;
    stream.write_all(&payload).await?;
    stream.flush().await?;
    
    // Read response header
    let mut header_buf = [0u8; FrameHeader::SIZE];
    stream.read_exact(&mut header_buf).await?;
    let resp_header = FrameHeader::from_bytes(header_buf);
    
    // Read response payload
    let mut resp_buf = vec![0u8; resp_header.length as usize];
    stream.read_exact(&mut resp_buf).await?;
    
    // Deserialize response
    let response: IpcResponse = serde_json::from_slice(&resp_buf)?;
    Ok(response)
}

// ========== Convenience functions ==========

/// Get service version
pub async fn get_version() -> IpcResult<IpcResponse> {
    send_request(&IpcRequest::GetVersion).await
}

/// Start the core with given config
pub async fn start_core(config: crate::CoreConfig) -> IpcResult<IpcResponse> {
    send_request(&IpcRequest::StartCore(config)).await
}

/// Stop the running core
pub async fn stop_core() -> IpcResult<IpcResponse> {
    send_request(&IpcRequest::StopCore).await
}

/// Restart the core
pub async fn restart_core() -> IpcResult<IpcResponse> {
    send_request(&IpcRequest::RestartCore).await
}

/// Reload config from file (restart core with new config)
pub async fn reload_config(config_path: &str) -> IpcResult<IpcResponse> {
    send_request(&IpcRequest::ReloadConfig {
        config_path: config_path.to_string(),
    }).await
}

/// Get core status
pub async fn get_status() -> IpcResult<IpcResponse> {
    send_request(&IpcRequest::GetStatus).await
}

/// Get logs
pub async fn get_logs(limit: Option<usize>) -> IpcResult<IpcResponse> {
    send_request(&IpcRequest::GetLogs { limit }).await
}

/// Clear logs
pub async fn clear_logs() -> IpcResult<IpcResponse> {
    send_request(&IpcRequest::ClearLogs).await
}

/// Check if core is running
pub async fn is_running() -> IpcResult<IpcResponse> {
    send_request(&IpcRequest::IsRunning).await
}

/// Ping the service
pub async fn ping() -> IpcResult<IpcResponse> {
    send_request(&IpcRequest::Ping).await
}

/// Check if service is available
pub async fn is_service_available() -> bool {
    ping().await.is_ok()
}

