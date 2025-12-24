//! AQiu Service - Background daemon for managing Mihomo core
//! 
//! This service runs as a privileged daemon (LaunchDaemon on macOS)
//! and manages the Mihomo core process, providing IPC communication
//! with the main AQiu application.

mod core_manager;
mod log_collector;
mod handler;

use aqiu_service_ipc::IpcServer;
use std::sync::Arc;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    init_logging()?;
    
    tracing::info!("AQiu Service v{} starting...", aqiu_service_ipc::VERSION);
    
    // Create core manager
    let core_manager = Arc::new(core_manager::CoreManager::new());
    
    // Create log collector
    let log_collector = Arc::new(log_collector::LogCollector::new(1000));
    
    // Create request handler
    let handler = Arc::new(handler::ServiceHandler::new(
        core_manager.clone(),
        log_collector.clone(),
    ));
    
    // Start IPC server
    let server = IpcServer::new(handler).await?;
    
    // Setup signal handlers for graceful shutdown
    let core_manager_clone = core_manager.clone();
    tokio::spawn(async move {
        if let Err(e) = wait_for_shutdown().await {
            tracing::error!("Signal handler error: {}", e);
        }
        tracing::info!("Shutdown signal received, stopping core...");
        core_manager_clone.stop().await;
        std::process::exit(0);
    });
    
    tracing::info!("AQiu Service ready, listening for connections");
    
    // Run server
    server.run().await?;
    
    Ok(())
}

fn init_logging() -> anyhow::Result<()> {
    // Log to /var/log/aqiu-service.log on macOS/Linux
    let log_dir = if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
        std::path::PathBuf::from("/var/log")
    } else {
        dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("aqiu")
    };
    
    let file_appender = tracing_appender::rolling::daily(&log_dir, "aqiu-service.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    
    // Keep guard alive for the lifetime of the program
    std::mem::forget(_guard);
    
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(fmt::layer().with_writer(non_blocking))
        .with(fmt::layer().with_writer(std::io::stderr))
        .init();
    
    Ok(())
}

#[cfg(unix)]
async fn wait_for_shutdown() -> anyhow::Result<()> {
    use tokio::signal::unix::{signal, SignalKind};
    
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;
    
    tokio::select! {
        _ = sigterm.recv() => {
            tracing::info!("Received SIGTERM");
        }
        _ = sigint.recv() => {
            tracing::info!("Received SIGINT");
        }
    }
    
    Ok(())
}

#[cfg(windows)]
async fn wait_for_shutdown() -> anyhow::Result<()> {
    tokio::signal::ctrl_c().await?;
    Ok(())
}

