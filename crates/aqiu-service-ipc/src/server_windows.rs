// Windows Named Pipes server implementation

#[cfg(windows)]
use crate::{IpcError, IpcRequest, IpcResponse, IpcResult, FrameHeader};
#[cfg(windows)]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, FromRawHandle};
#[cfg(windows)]
use std::ptr;

#[cfg(windows)]
const PIPE_NAME: &str = r"\\.\pipe\aqiu-service";

#[cfg(windows)]
const PIPE_ACCESS_DUPLEX: u32 = 0x00000003;
#[cfg(windows)]
const FILE_FLAG_OVERLAPPED: u32 = 0x40000000;
#[cfg(windows)]
const PIPE_TYPE_BYTE: u32 = 0x00000000;
#[cfg(windows)]
const PIPE_READMODE_BYTE: u32 = 0x00000000;
#[cfg(windows)]
const PIPE_WAIT: u32 = 0x00000000;
#[cfg(windows)]
const PIPE_UNLIMITED_INSTANCES: u32 = 255;

/// Windows Named Pipe Server
#[cfg(windows)]
pub struct NamedPipeServer {
    pipe_name: String,
}

#[cfg(windows)]
impl NamedPipeServer {
    pub fn new() -> Self {
        Self {
            pipe_name: PIPE_NAME.to_string(),
        }
    }

    /// Create a new named pipe instance
    fn create_pipe_instance(&self) -> IpcResult<tokio::fs::File> {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        
        let pipe_name_wide: Vec<u16> = OsStr::new(&self.pipe_name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let handle = unsafe {
            winapi::um::winbase::CreateNamedPipeW(
                pipe_name_wide.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                4096, // output buffer size
                4096, // input buffer size
                0,    // default timeout
                ptr::null_mut(),
            )
        };

        if handle == winapi::um::handleapi::INVALID_HANDLE_VALUE {
            return Err(IpcError::ConnectionFailed(
                "Failed to create named pipe".to_string(),
            ));
        }

        // Convert to tokio File
        let tokio_file = unsafe { tokio::fs::File::from_raw_handle(handle) };
        Ok(tokio_file)
    }

    /// Wait for a client to connect
    async fn wait_for_connection(&self, pipe: &mut tokio::fs::File) -> IpcResult<()> {
        use std::os::windows::io::AsRawHandle;
        
        let handle = pipe.as_raw_handle();
        
        // ConnectNamedPipe is synchronous, but we can use it in a blocking task
        let result = tokio::task::spawn_blocking(move || unsafe {
            winapi::um::namedpipeapi::ConnectNamedPipe(handle, ptr::null_mut())
        })
        .await
        .map_err(|e| IpcError::ConnectionFailed(e.to_string()))?;

        if result == 0 {
            let error = unsafe { winapi::um::errhandlingapi::GetLastError() };
            // ERROR_PIPE_CONNECTED means client is already connected
            if error != winapi::shared::winerror::ERROR_PIPE_CONNECTED {
                return Err(IpcError::ConnectionFailed(format!(
                    "ConnectNamedPipe failed with error: {}",
                    error
                )));
            }
        }

        Ok(())
    }

    /// Handle a single client connection
    async fn handle_client<F, Fut>(
        &self,
        mut pipe: tokio::fs::File,
        handler: F,
    ) -> IpcResult<()>
    where
        F: Fn(IpcRequest) -> Fut,
        Fut: std::future::Future<Output = IpcResponse>,
    {
        loop {
            // Read request header
            let mut header_buf = [0u8; FrameHeader::SIZE];
            match pipe.read_exact(&mut header_buf).await {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    // Client disconnected
                    break;
                }
                Err(e) => {
                    tracing::error!("Failed to read header: {}", e);
                    break;
                }
            }

            let header = FrameHeader::from_bytes(header_buf);

            // Read request payload
            let mut payload_buf = vec![0u8; header.length as usize];
            if let Err(e) = pipe.read_exact(&mut payload_buf).await {
                tracing::error!("Failed to read payload: {}", e);
                break;
            }

            // Deserialize request
            let request: IpcRequest = match serde_json::from_slice(&payload_buf) {
                Ok(req) => req,
                Err(e) => {
                    tracing::error!("Failed to deserialize request: {}", e);
                    break;
                }
            };

            // Handle request
            let response = handler(request).await;

            // Serialize response
            let response_payload = match serde_json::to_vec(&response) {
                Ok(payload) => payload,
                Err(e) => {
                    tracing::error!("Failed to serialize response: {}", e);
                    break;
                }
            };

            let response_header = FrameHeader::new(response_payload.len() as u32);

            // Write response
            if let Err(e) = pipe.write_all(&response_header.to_bytes()).await {
                tracing::error!("Failed to write response header: {}", e);
                break;
            }

            if let Err(e) = pipe.write_all(&response_payload).await {
                tracing::error!("Failed to write response payload: {}", e);
                break;
            }

            if let Err(e) = pipe.flush().await {
                tracing::error!("Failed to flush pipe: {}", e);
                break;
            }
        }

        Ok(())
    }

    /// Run the server
    pub async fn run<F, Fut>(&self, handler: F) -> IpcResult<()>
    where
        F: Fn(IpcRequest) -> Fut + Clone + Send + 'static,
        Fut: std::future::Future<Output = IpcResponse> + Send + 'static,
    {
        tracing::info!("Starting Windows Named Pipe server at {}", self.pipe_name);

        loop {
            // Create a new pipe instance
            let mut pipe = self.create_pipe_instance()?;

            // Wait for a client to connect
            if let Err(e) = self.wait_for_connection(&mut pipe).await {
                tracing::error!("Failed to wait for connection: {}", e);
                continue;
            }

            tracing::info!("Client connected");

            // Handle the client in a separate task
            let handler_clone = handler.clone();
            tokio::spawn(async move {
                if let Err(e) = NamedPipeServer::new()
                    .handle_client(pipe, handler_clone)
                    .await
                {
                    tracing::error!("Error handling client: {}", e);
                }
                tracing::info!("Client disconnected");
            });
        }
    }
}

// Stub implementation for non-Windows platforms
#[cfg(not(windows))]
pub struct NamedPipeServer;

#[cfg(not(windows))]
impl NamedPipeServer {
    pub fn new() -> Self {
        Self
    }

    pub async fn run<F, Fut>(&self, _handler: F) -> IpcResult<()>
    where
        F: Fn(IpcRequest) -> Fut + Clone + Send + 'static,
        Fut: std::future::Future<Output = IpcResponse> + Send + 'static,
    {
        Err(IpcError::ConnectionFailed(
            "Windows Named Pipes only available on Windows".to_string(),
        ))
    }
}
