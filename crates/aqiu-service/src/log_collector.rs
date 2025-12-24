//! Log Collector - Collects and stores core logs

use aqiu_service_ipc::LogEntry;
use parking_lot::RwLock;
use std::collections::VecDeque;
use tokio::sync::mpsc;
use crate::core_manager::LogLine;

/// Log Collector - collects logs from core output
pub struct LogCollector {
    /// Stored log entries
    logs: RwLock<VecDeque<LogEntry>>,
    /// Maximum number of logs to store
    max_size: usize,
    /// Receiver for log lines
    receiver: RwLock<Option<mpsc::UnboundedReceiver<LogLine>>>,
}

impl LogCollector {
    pub fn new(max_size: usize) -> Self {
        Self {
            logs: RwLock::new(VecDeque::with_capacity(max_size)),
            max_size,
            receiver: RwLock::new(None),
        }
    }
    
    /// Create a sender for forwarding logs
    pub fn create_sender(&self) -> mpsc::UnboundedSender<LogLine> {
        let (sender, receiver) = mpsc::unbounded_channel();
        *self.receiver.write() = Some(receiver);
        sender
    }
    
    /// Start processing incoming logs
    pub async fn start_processing(&self) {
        let receiver = self.receiver.write().take();
        
        if let Some(mut rx) = receiver {
            while let Some(log_line) = rx.recv().await {
                self.add_log(LogEntry {
                    timestamp: log_line.timestamp.to_rfc3339(),
                    level: log_line.level,
                    message: log_line.message,
                });
            }
        }
    }
    
    /// Add a log entry
    pub fn add_log(&self, entry: LogEntry) {
        let mut logs = self.logs.write();
        
        if logs.len() >= self.max_size {
            logs.pop_front();
        }
        
        logs.push_back(entry);
    }
    
    /// Get logs with optional limit
    pub fn get_logs(&self, limit: Option<usize>) -> Vec<LogEntry> {
        let logs = self.logs.read();
        
        match limit {
            Some(n) => logs.iter().rev().take(n).rev().cloned().collect(),
            None => logs.iter().cloned().collect(),
        }
    }
    
    /// Clear all logs
    pub fn clear(&self) {
        self.logs.write().clear();
    }
    
    /// Get log count
    pub fn count(&self) -> usize {
        self.logs.read().len()
    }
}

