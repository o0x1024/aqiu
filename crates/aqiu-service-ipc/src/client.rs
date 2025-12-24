//! IPC Client implementation
//! 
//! Provides async client for connecting to aqiu-service daemon.
//! - Unix: Uses Unix Domain Sockets
//! - Windows: Uses Named Pipes

use crate::{IpcError, IpcRequest, IpcResponse, IpcResult, FrameHeader};
use tokio::time::Duration;

#[cfg(unix)]
use crate::IPC_PATH;
#[cfg(unix)]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(unix)]
use tokio::net::UnixStream;
#[cfg(unix)]
use tokio::time::timeout;

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

// ========== Unix Socket Implementation ==========

/// Connect to the service and return a stream
#[cfg(unix)]
async fn connect() -> IpcResult<UnixStream> {
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
async fn send_request_impl(request: &IpcRequest) -> IpcResult<IpcResponse> {
    send_request_with_config_impl(request, &IpcConfig::default()).await
}

/// Send a request with custom config
#[cfg(unix)]
async fn send_request_with_config_impl(
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

// ========== Windows Named Pipes Implementation ==========

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::time::timeout;
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::{AsRawHandle, FromRawHandle};

    const PIPE_NAME: &str = r"\\.\pipe\aqiu-service";
    const FILE_FLAG_OVERLAPPED: u32 = 0x40000000;

    async fn connect() -> IpcResult<tokio::fs::File> {
        // Try to open the named pipe
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(FILE_FLAG_OVERLAPPED)
            .open(PIPE_NAME)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    IpcError::ServiceUnavailable
                } else {
                    IpcError::ConnectionFailed(e.to_string())
                }
            })?;
        
        // Convert to tokio File for async operations
        let handle = file.as_raw_handle();
        std::mem::forget(file); // Prevent closing the handle
        
        let tokio_file = unsafe { tokio::fs::File::from_raw_handle(handle) };
        
        Ok(tokio_file)
    }

    pub(super) async fn send_request_impl(request: &IpcRequest) -> IpcResult<IpcResponse> {
        send_request_with_config_impl(request, &IpcConfig::default()).await
    }

    pub(super) async fn send_request_with_config_impl(
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

    async fn send_request_inner(request: &IpcRequest) -> IpcResult<IpcResponse> {
        let mut pipe = connect().await?;
        
        // Serialize request
        let payload = serde_json::to_vec(request)?;
        let header = FrameHeader::new(payload.len() as u32);
        
        // Write header + payload
        pipe.write_all(&header.to_bytes()).await?;
        pipe.write_all(&payload).await?;
        pipe.flush().await?;
        
        // Read response header
        let mut header_buf = [0u8; FrameHeader::SIZE];
        pipe.read_exact(&mut header_buf).await?;
        let resp_header = FrameHeader::from_bytes(header_buf);
        
        // Read response payload
        let mut resp_buf = vec![0u8; resp_header.length as usize];
        pipe.read_exact(&mut resp_buf).await?;
        
        // Deserialize response
        let response: IpcResponse = serde_json::from_slice(&resp_buf)?;
        Ok(response)
    }
}

// ========== Public API (platform-agnostic) ==========

/// Send a request and receive a response
pub async fn send_request(request: &IpcRequest) -> IpcResult<IpcResponse> {
    #[cfg(unix)]
    return send_request_impl(request).await;
    
    #[cfg(windows)]
    return windows_impl::send_request_impl(request).await;
    
    #[cfg(not(any(unix, windows)))]
    return Err(IpcError::ConnectionFailed("Unsupported platform".to_string()));
}

/// Send a request with custom config
pub async fn send_request_with_config(
    request: &IpcRequest,
    config: &IpcConfig,
) -> IpcResult<IpcResponse> {
    #[cfg(unix)]
    return send_request_with_config_impl(request, config).await;
    
    #[cfg(windows)]
    return windows_impl::send_request_with_config_impl(request, config).await;
    
    #[cfg(not(any(unix, windows)))]
    return Err(IpcError::ConnectionFailed("Unsupported platform".to_string()));
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

