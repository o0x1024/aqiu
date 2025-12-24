//! Core Manager - Manages the Mihomo core process

use aqiu_service_ipc::{CoreConfig, CoreStatus};
use parking_lot::RwLock;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

/// Log line from core output
#[derive(Debug, Clone)]
pub struct LogLine {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: String,
    pub message: String,
}

/// Core Manager - manages the mihomo process lifecycle
pub struct CoreManager {
    /// Current running process
    process: RwLock<Option<Child>>,
    /// Current PID
    pid: RwLock<Option<u32>>,
    /// Start time
    start_time: RwLock<Option<Instant>>,
    /// Current config
    config: RwLock<Option<CoreConfig>>,
    /// Last error
    last_error: RwLock<Option<String>>,
    /// Log sender
    log_sender: RwLock<Option<mpsc::UnboundedSender<LogLine>>>,
}

impl CoreManager {
    pub fn new() -> Self {
        Self {
            process: RwLock::new(None),
            pid: RwLock::new(None),
            start_time: RwLock::new(None),
            config: RwLock::new(None),
            last_error: RwLock::new(None),
            log_sender: RwLock::new(None),
        }
    }
    
    /// Set log sender for forwarding core output
    pub fn set_log_sender(&self, sender: mpsc::UnboundedSender<LogLine>) {
        *self.log_sender.write() = Some(sender);
    }
    
    /// Start the core with given config
    pub async fn start(&self, config: CoreConfig) -> Result<(), String> {
        // Stop any existing process first
        self.stop().await;
        
        tracing::info!("Starting core: {}", config.core_path);
        tracing::info!("Config: {}", config.config_path);
        tracing::info!("Working dir: {}", config.config_dir);
        
        // Validate paths
        if !std::path::Path::new(&config.core_path).exists() {
            let err = format!("Core binary not found: {}", config.core_path);
            *self.last_error.write() = Some(err.clone());
            return Err(err);
        }
        
        if !std::path::Path::new(&config.config_path).exists() {
            let err = format!("Config file not found: {}", config.config_path);
            *self.last_error.write() = Some(err.clone());
            return Err(err);
        }
        
        // Start the core process
        let mut cmd = Command::new(&config.core_path);
        cmd.arg("-d").arg(&config.config_dir)
           .arg("-f").arg(&config.config_path)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped())
           .kill_on_drop(true);
        
        match cmd.spawn() {
            Ok(mut child) => {
                let pid = child.id();
                tracing::info!("Core started with PID: {:?}", pid);
                
                // Capture stdout
                if let Some(stdout) = child.stdout.take() {
                    let sender = self.log_sender.read().clone();
                    tokio::spawn(async move {
                        Self::process_output(stdout, "INFO", sender).await;
                    });
                }
                
                // Capture stderr
                if let Some(stderr) = child.stderr.take() {
                    let sender = self.log_sender.read().clone();
                    tokio::spawn(async move {
                        Self::process_output(stderr, "ERROR", sender).await;
                    });
                }
                
                *self.process.write() = Some(child);
                *self.pid.write() = pid;
                *self.start_time.write() = Some(Instant::now());
                *self.config.write() = Some(config);
                *self.last_error.write() = None;
                
                Ok(())
            }
            Err(e) => {
                let err = format!("Failed to start core: {}", e);
                tracing::error!("{}", err);
                *self.last_error.write() = Some(err.clone());
                Err(err)
            }
        }
    }
    
    async fn process_output<R: tokio::io::AsyncRead + Unpin>(
        reader: R,
        default_level: &str,
        sender: Option<mpsc::UnboundedSender<LogLine>>,
    ) {
        let mut lines = BufReader::new(reader).lines();
        
        while let Ok(Some(line)) = lines.next_line().await {
            // Parse log level from line if present
            let (level, message) = if line.contains("level=debug") || line.contains("[DEBUG]") {
                ("DEBUG", line.clone())
            } else if line.contains("level=info") || line.contains("[INFO]") {
                ("INFO", line.clone())
            } else if line.contains("level=warn") || line.contains("[WARN]") {
                ("WARN", line.clone())
            } else if line.contains("level=error") || line.contains("[ERROR]") {
                ("ERROR", line.clone())
            } else {
                (default_level, line.clone())
            };
            
            let log_line = LogLine {
                timestamp: chrono::Utc::now(),
                level: level.to_string(),
                message,
            };
            
            // Forward to log collector
            if let Some(ref sender) = sender {
                let _ = sender.send(log_line);
            }
        }
    }
    
    /// Stop the core
    pub async fn stop(&self) {
        if let Some(mut process) = self.process.write().take() {
            tracing::info!("Stopping core process...");
            
            // Try graceful kill first
            if let Err(e) = process.kill().await {
                tracing::warn!("Failed to kill process: {}", e);
            }
            
            // Wait for process to exit
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                process.wait()
            ).await {
                Ok(Ok(status)) => {
                    tracing::info!("Core process exited with status: {}", status);
                }
                Ok(Err(e)) => {
                    tracing::error!("Error waiting for process: {}", e);
                }
                Err(_) => {
                    tracing::warn!("Timeout waiting for process to exit");
                }
            }
        }
        
        *self.pid.write() = None;
        *self.start_time.write() = None;
    }
    
    /// Restart the core with current config
    pub async fn restart(&self) -> Result<(), String> {
        let config = self.config.read().clone();
        
        match config {
            Some(cfg) => {
                self.stop().await;
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                self.start(cfg).await
            }
            None => Err("No config available for restart".to_string())
        }
    }
    
    /// Reload config from a new path (restart core with new config)
    pub async fn reload_config(&self, config_path: &str) -> Result<(), String> {
        let mut config = self.config.read().clone()
            .ok_or_else(|| "No config available, start core first".to_string())?;
        
        // Update config path
        config.config_path = config_path.to_string();
        
        tracing::info!("Reloading config from: {}", config_path);
        
        // Stop and restart with new config
        self.stop().await;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        self.start(config).await
    }
    
    /// Check if core is running
    pub fn is_running(&self) -> bool {
        self.process.read().is_some() && self.pid.read().is_some()
    }
    
    /// Get current status
    pub fn status(&self) -> CoreStatus {
        let running = self.is_running();
        let pid = *self.pid.read();
        let uptime = self.start_time.read()
            .map(|t| t.elapsed().as_secs());
        let config_path = self.config.read()
            .as_ref()
            .map(|c| c.config_path.clone());
        let last_error = self.last_error.read().clone();
        
        CoreStatus {
            running,
            pid,
            uptime_secs: uptime,
            config_path,
            last_error,
        }
    }
}

impl Default for CoreManager {
    fn default() -> Self {
        Self::new()
    }
}

