//! IPC Server implementation
//! 
//! Provides async server for the aqiu-service daemon.

use crate::{IpcError, IpcRequest, IpcResponse, IpcResult, FrameHeader, IPC_PATH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use std::sync::Arc;

/// Trait for handling IPC requests
#[async_trait::async_trait]
pub trait RequestHandler: Send + Sync {
    /// Handle an incoming request
    async fn handle(&self, request: IpcRequest) -> IpcResponse;
}

/// IPC Server
pub struct IpcServer {
    listener: UnixListener,
    handler: Arc<dyn RequestHandler>,
}

impl IpcServer {
    /// Create a new IPC server
    pub async fn new(handler: Arc<dyn RequestHandler>) -> IpcResult<Self> {
        // Remove existing socket file if it exists
        let _ = std::fs::remove_file(IPC_PATH);
        
        // Create parent directory if needed
        if let Some(parent) = std::path::Path::new(IPC_PATH).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        
        let listener = UnixListener::bind(IPC_PATH)
            .map_err(|e| IpcError::ConnectionFailed(e.to_string()))?;
        
        // Set socket permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o660);
            let _ = std::fs::set_permissions(IPC_PATH, perms);
        }
        
        tracing::info!("IPC server listening on {}", IPC_PATH);
        
        Ok(Self { listener, handler })
    }
    
    /// Run the server (blocking)
    pub async fn run(&self) -> IpcResult<()> {
        loop {
            match self.listener.accept().await {
                Ok((stream, _)) => {
                    let handler = self.handler.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, handler).await {
                            tracing::error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Accept error: {}", e);
                }
            }
        }
    }
    
    async fn handle_connection(
        mut stream: tokio::net::UnixStream,
        handler: Arc<dyn RequestHandler>,
    ) -> IpcResult<()> {
        // Read request header
        let mut header_buf = [0u8; FrameHeader::SIZE];
        stream.read_exact(&mut header_buf).await?;
        let header = FrameHeader::from_bytes(header_buf);
        
        // Validate payload size (max 10MB)
        if header.length > 10 * 1024 * 1024 {
            let response = IpcResponse::error(400, "Payload too large");
            Self::send_response(&mut stream, &response).await?;
            return Ok(());
        }
        
        // Read request payload
        let mut payload = vec![0u8; header.length as usize];
        stream.read_exact(&mut payload).await?;
        
        // Deserialize request
        let request: IpcRequest = match serde_json::from_slice(&payload) {
            Ok(req) => req,
            Err(e) => {
                let response = IpcResponse::error(400, format!("Invalid request: {}", e));
                Self::send_response(&mut stream, &response).await?;
                return Ok(());
            }
        };
        
        tracing::debug!("Received request: {:?}", request);
        
        // Handle request
        let response = handler.handle(request).await;
        
        // Send response
        Self::send_response(&mut stream, &response).await?;
        
        Ok(())
    }
    
    async fn send_response(
        stream: &mut tokio::net::UnixStream,
        response: &IpcResponse,
    ) -> IpcResult<()> {
        let payload = serde_json::to_vec(response)?;
        let header = FrameHeader::new(payload.len() as u32);
        
        stream.write_all(&header.to_bytes()).await?;
        stream.write_all(&payload).await?;
        stream.flush().await?;
        
        Ok(())
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(IPC_PATH);
    }
}

