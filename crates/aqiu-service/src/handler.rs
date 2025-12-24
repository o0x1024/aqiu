//! Request Handler - Handles IPC requests

use aqiu_service_ipc::{IpcRequest, IpcResponse, ResponseData, VERSION};
use async_trait::async_trait;
use std::sync::Arc;

use crate::core_manager::CoreManager;
use crate::log_collector::LogCollector;

/// Service request handler
pub struct ServiceHandler {
    core_manager: Arc<CoreManager>,
    log_collector: Arc<LogCollector>,
}

impl ServiceHandler {
    pub fn new(
        core_manager: Arc<CoreManager>,
        log_collector: Arc<LogCollector>,
    ) -> Self {
        // Set up log forwarding
        let sender = log_collector.create_sender();
        core_manager.set_log_sender(sender);
        
        // Start log processing in background
        let collector = log_collector.clone();
        tokio::spawn(async move {
            collector.start_processing().await;
        });
        
        Self {
            core_manager,
            log_collector,
        }
    }
}

#[async_trait]
impl aqiu_service_ipc::RequestHandler for ServiceHandler {
    async fn handle(&self, request: IpcRequest) -> IpcResponse {
        match request {
            IpcRequest::GetVersion => {
                IpcResponse::success_with_data(
                    "Version retrieved",
                    ResponseData::Version(VERSION.to_string()),
                )
            }
            
            IpcRequest::StartCore(config) => {
                tracing::info!("Starting core with config: {:?}", config);
                
                match self.core_manager.start(config).await {
                    Ok(()) => IpcResponse::success("Core started successfully"),
                    Err(e) => IpcResponse::error(1, e),
                }
            }
            
            IpcRequest::StopCore => {
                tracing::info!("Stopping core");
                self.core_manager.stop().await;
                IpcResponse::success("Core stopped")
            }
            
            IpcRequest::RestartCore => {
                tracing::info!("Restarting core");
                
                match self.core_manager.restart().await {
                    Ok(()) => IpcResponse::success("Core restarted successfully"),
                    Err(e) => IpcResponse::error(1, e),
                }
            }
            
            IpcRequest::ReloadConfig { config_path } => {
                tracing::info!("Reloading config from: {}", config_path);
                
                match self.core_manager.reload_config(&config_path).await {
                    Ok(()) => IpcResponse::success("Config reloaded successfully"),
                    Err(e) => IpcResponse::error(1, e),
                }
            }
            
            IpcRequest::GetStatus => {
                let status = self.core_manager.status();
                IpcResponse::success_with_data(
                    "Status retrieved",
                    ResponseData::Status(status),
                )
            }
            
            IpcRequest::GetLogs { limit } => {
                let logs = self.log_collector.get_logs(limit);
                IpcResponse::success_with_data(
                    format!("Retrieved {} logs", logs.len()),
                    ResponseData::Logs(logs),
                )
            }
            
            IpcRequest::ClearLogs => {
                self.log_collector.clear();
                IpcResponse::success("Logs cleared")
            }
            
            IpcRequest::IsRunning => {
                let running = self.core_manager.is_running();
                IpcResponse::success_with_data(
                    if running { "Core is running" } else { "Core is not running" },
                    ResponseData::Bool(running),
                )
            }
            
            IpcRequest::Ping => {
                IpcResponse::success_with_data("Pong", ResponseData::Pong)
            }
            
            IpcRequest::Shutdown => {
                tracing::info!("Shutdown requested");
                
                // Stop core first
                self.core_manager.stop().await;
                
                // Schedule shutdown
                tokio::spawn(async {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    std::process::exit(0);
                });
                
                IpcResponse::success("Shutting down")
            }
        }
    }
}

