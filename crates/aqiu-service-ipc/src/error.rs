//! IPC Error types

use thiserror::Error;

/// IPC Error types
#[derive(Error, Debug)]
pub enum IpcError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Connection closed")]
    ConnectionClosed,
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    #[error("Timeout")]
    Timeout,
    
    #[error("Service unavailable")]
    ServiceUnavailable,
    
    #[error("Version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: String, actual: String },
    
    #[error("Request failed: {0}")]
    RequestFailed(String),
}

/// Result type alias
pub type IpcResult<T> = Result<T, IpcError>;

