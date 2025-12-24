//! IPC Protocol definitions
//! 
//! Defines the request/response protocol for communication between
//! the main app and the service daemon.

use serde::{Deserialize, Serialize};

/// Core configuration for starting mihomo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    /// Path to the config file
    pub config_path: String,
    /// Path to the mihomo binary
    pub core_path: String,
    /// Working directory for the core
    pub config_dir: String,
}

/// Request types sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum IpcRequest {
    /// Get service version
    GetVersion,
    
    /// Start the mihomo core with given config
    StartCore(CoreConfig),
    
    /// Stop the running mihomo core
    StopCore,
    
    /// Restart the mihomo core (stop and start with same config)
    RestartCore,
    
    /// Reload config from file (restart core to apply new config)
    ReloadConfig {
        /// Path to the config file to reload
        config_path: String,
    },
    
    /// Get current core status
    GetStatus,
    
    /// Get collected logs
    GetLogs { 
        /// Maximum number of log lines to return
        limit: Option<usize> 
    },
    
    /// Clear collected logs
    ClearLogs,
    
    /// Check if core is running
    IsRunning,
    
    /// Ping - for connection testing
    Ping,
    
    /// Shutdown the service (admin only)
    Shutdown,
}

/// Core running status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreStatus {
    /// Whether the core is running
    pub running: bool,
    /// PID of the core process (if running)
    pub pid: Option<u32>,
    /// Uptime in seconds (if running)
    pub uptime_secs: Option<u64>,
    /// Current config path (if running)
    pub config_path: Option<String>,
    /// Last error message (if any)
    pub last_error: Option<String>,
}

/// Log entry from the core
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Timestamp (ISO 8601)
    pub timestamp: String,
    /// Log level (INFO, WARN, ERROR, DEBUG)
    pub level: String,
    /// Log message
    pub message: String,
}

/// Response types sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
    /// Response code (0 = success, >0 = error)
    pub code: i32,
    /// Human-readable message
    pub message: String,
    /// Optional data payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<ResponseData>,
}

/// Response data variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponseData {
    /// Version string
    Version(String),
    /// Core status
    Status(CoreStatus),
    /// Log entries
    Logs(Vec<LogEntry>),
    /// Simple boolean
    Bool(bool),
    /// Pong response
    Pong,
}

impl IpcResponse {
    /// Create a success response
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            code: 0,
            message: message.into(),
            data: None,
        }
    }
    
    /// Create a success response with data
    pub fn success_with_data(message: impl Into<String>, data: ResponseData) -> Self {
        Self {
            code: 0,
            message: message.into(),
            data: Some(data),
        }
    }
    
    /// Create an error response
    pub fn error(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }
    
    /// Check if response is successful
    pub fn is_success(&self) -> bool {
        self.code == 0
    }
}

/// Frame header for length-prefixed messages
#[derive(Debug, Clone, Copy)]
pub struct FrameHeader {
    /// Length of the payload in bytes
    pub length: u32,
}

impl FrameHeader {
    /// Header size in bytes
    pub const SIZE: usize = 4;
    
    /// Create a new frame header
    pub fn new(length: u32) -> Self {
        Self { length }
    }
    
    /// Encode to bytes
    pub fn to_bytes(&self) -> [u8; 4] {
        self.length.to_be_bytes()
    }
    
    /// Decode from bytes
    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        Self {
            length: u32::from_be_bytes(bytes),
        }
    }
}

