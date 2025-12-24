//! AQiu Service IPC - Unix Socket communication protocol
//! 
//! This crate provides the IPC protocol and client/server implementations
//! for communication between the main AQiu app and the aqiu-service daemon.

mod protocol;
mod error;

#[cfg(feature = "client")]
mod client;

#[cfg(feature = "server")]
mod server;

#[cfg(all(feature = "server", windows))]
mod server_windows;

pub use protocol::*;
pub use error::*;

#[cfg(feature = "client")]
pub use client::*;

#[cfg(feature = "server")]
pub use server::*;

#[cfg(all(feature = "server", windows))]
pub use server_windows::*;

/// IPC socket path
#[cfg(target_os = "macos")]
pub const IPC_PATH: &str = "/var/run/aqiu-service.sock";

#[cfg(target_os = "linux")]
pub const IPC_PATH: &str = "/var/run/aqiu-service.sock";

#[cfg(target_os = "windows")]
pub const IPC_PATH: &str = r"\\.\pipe\aqiu-service";

/// Service version - must match between client and server
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

