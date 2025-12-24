//! Service IPC Client
//! 
//! Provides interface for communicating with aqiu-service daemon.

#![allow(dead_code)]

use aqiu_service_ipc::{
    CoreConfig, CoreStatus, ResponseData, IPC_PATH, VERSION,
};
use std::path::Path;

/// Check if service is available
pub async fn is_service_available() -> bool {
    aqiu_service_ipc::is_service_available().await
}

/// Check if service socket exists
pub fn is_service_socket_exists() -> bool {
    Path::new(IPC_PATH).exists()
}

/// Get service version
pub async fn get_version() -> Result<String, String> {
    let response = aqiu_service_ipc::get_version()
        .await
        .map_err(|e| e.to_string())?;
    
    if !response.is_success() {
        return Err(response.message);
    }
    
    match response.data {
        Some(ResponseData::Version(v)) => Ok(v),
        _ => Err("Invalid response data".to_string()),
    }
}

/// Check if service version matches
pub async fn check_version_match() -> Result<bool, String> {
    let version = get_version().await?;
    Ok(version == VERSION)
}

/// Start core via service
#[allow(dead_code)]
pub async fn start_core(
    config_path: &str,
    core_path: &str,
    config_dir: &str,
) -> Result<(), String> {
    let config = CoreConfig {
        config_path: config_path.to_string(),
        core_path: core_path.to_string(),
        config_dir: config_dir.to_string(),
    };
    
    let response = aqiu_service_ipc::start_core(config)
        .await
        .map_err(|e| e.to_string())?;
    
    if response.is_success() {
        Ok(())
    } else {
        Err(response.message)
    }
}

/// Stop core via service
#[allow(dead_code)]
pub async fn stop_core() -> Result<(), String> {
    let response = aqiu_service_ipc::stop_core()
        .await
        .map_err(|e| e.to_string())?;
    
    if response.is_success() {
        Ok(())
    } else {
        Err(response.message)
    }
}

/// Restart core via service
#[allow(dead_code)]
pub async fn restart_core() -> Result<(), String> {
    let response = aqiu_service_ipc::restart_core()
        .await
        .map_err(|e| e.to_string())?;
    
    if response.is_success() {
        Ok(())
    } else {
        Err(response.message)
    }
}

/// Get core status via service
#[allow(dead_code)]
pub async fn get_status() -> Result<CoreStatus, String> {
    let response = aqiu_service_ipc::get_status()
        .await
        .map_err(|e| e.to_string())?;
    
    if !response.is_success() {
        return Err(response.message);
    }
    
    match response.data {
        Some(ResponseData::Status(status)) => Ok(status),
        _ => Err("Invalid response data".to_string()),
    }
}

/// Check if core is running via service
#[allow(dead_code)]
pub async fn is_running() -> Result<bool, String> {
    let response = aqiu_service_ipc::is_running()
        .await
        .map_err(|e| e.to_string())?;
    
    if !response.is_success() {
        return Err(response.message);
    }
    
    match response.data {
        Some(ResponseData::Bool(running)) => Ok(running),
        _ => Err("Invalid response data".to_string()),
    }
}

/// Get logs from service
#[allow(dead_code)]
pub async fn get_logs(limit: Option<usize>) -> Result<Vec<aqiu_service_ipc::LogEntry>, String> {
    let response = aqiu_service_ipc::get_logs(limit)
        .await
        .map_err(|e| e.to_string())?;
    
    if !response.is_success() {
        return Err(response.message);
    }
    
    match response.data {
        Some(ResponseData::Logs(logs)) => Ok(logs),
        _ => Err("Invalid response data".to_string()),
    }
}

/// Clear logs in service
#[allow(dead_code)]
pub async fn clear_logs() -> Result<(), String> {
    let response = aqiu_service_ipc::clear_logs()
        .await
        .map_err(|e| e.to_string())?;
    
    if response.is_success() {
        Ok(())
    } else {
        Err(response.message)
    }
}

/// Ping service
#[allow(dead_code)]
pub async fn ping() -> Result<(), String> {
    let response = aqiu_service_ipc::ping()
        .await
        .map_err(|e| e.to_string())?;
    
    if response.is_success() {
        Ok(())
    } else {
        Err(response.message)
    }
}

/// Service status enum
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceStatus {
    /// Service is ready and version matches
    Ready,
    /// Service needs reinstall (version mismatch)
    NeedsReinstall,
    /// Service is not installed
    NotInstalled,
    /// Service is unavailable
    Unavailable(String),
}

/// Check comprehensive service status
pub async fn check_service_status() -> ServiceStatus {
    // First check if socket exists
    if !is_service_socket_exists() {
        return ServiceStatus::NotInstalled;
    }
    
    // Try to ping
    if !is_service_available().await {
        return ServiceStatus::NotInstalled;
    }
    
    // Check version
    match check_version_match().await {
        Ok(true) => ServiceStatus::Ready,
        Ok(false) => ServiceStatus::NeedsReinstall,
        Err(e) => ServiceStatus::Unavailable(e),
    }
}

/// Install service using the install script
/// NOTE: This is the ONLY place that requires admin password (one-time setup)
#[cfg(target_os = "macos")]
pub async fn install_service(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    use std::process::Command as StdCommand;
    
    // Get the path to install script from resources
    let resource_dir = app.path().resource_dir()
        .map_err(|e| format!("Cannot get resource directory: {}", e))?;
    
    let mut install_script = resource_dir.join("aqiu-service-install.sh");
    
    if !install_script.exists() {
        // Try executable directory as fallback
        let exe_dir = std::env::current_exe()
            .map_err(|e| e.to_string())?
            .parent()
            .ok_or("Cannot get app directory")?
            .to_path_buf();
        
        install_script = exe_dir.join("aqiu-service-install.sh");
        if !install_script.exists() {
            return Err(format!("Install script not found in {:?} or {:?}", resource_dir, exe_dir));
        }
    }
    
    println!("Installing service using script: {:?}", install_script);
    
    // Use osascript with AppleScript to show native macOS authorization dialog
    // Use /bin/bash to execute script since it may not have +x permission
    let install_shell = install_script.to_string_lossy();
    let prompt = "AQiu needs administrator privileges to install Service.";
    let apple_script = format!(
        r#"do shell script "/bin/bash '{}'" with administrator privileges with prompt "{}""#,
        install_shell, prompt
    );
    
    let output = StdCommand::new("osascript")
        .args(["-e", &apple_script])
        .output()
        .map_err(|e| format!("Failed to run install script: {}", e))?;
    
    if output.status.success() {
        // Wait for service to start
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        println!("Service installed successfully");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Check if user cancelled the authorization
        if stderr.contains("User canceled") || stderr.contains("-128") {
            return Err("Authorization cancelled by user".to_string());
        }
        Err(format!("Install failed: {}", stderr))
    }
}

/// Uninstall service
/// NOTE: This is the ONLY place that requires admin password
#[cfg(target_os = "macos")]
pub async fn uninstall_service(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    use std::process::Command as StdCommand;
    
    let resource_dir = app.path().resource_dir()
        .map_err(|e| format!("Cannot get resource directory: {}", e))?;
    
    let mut uninstall_script = resource_dir.join("aqiu-service-uninstall.sh");
    
    if !uninstall_script.exists() {
        let exe_dir = std::env::current_exe()
            .map_err(|e| e.to_string())?
            .parent()
            .ok_or("Cannot get app directory")?
            .to_path_buf();
        
        uninstall_script = exe_dir.join("aqiu-service-uninstall.sh");
        if !uninstall_script.exists() {
            return Err(format!("Uninstall script not found"));
        }
    }
    
    println!("Uninstalling service using script: {:?}", uninstall_script);
    
    // Use osascript with AppleScript to show native macOS authorization dialog
    // Use /bin/bash to execute script since it may not have +x permission
    let uninstall_shell = uninstall_script.to_string_lossy();
    let prompt = "AQiu needs administrator privileges to uninstall Service.";
    let apple_script = format!(
        r#"do shell script "/bin/bash '{}'" with administrator privileges with prompt "{}""#,
        uninstall_shell, prompt
    );
    
    let output = StdCommand::new("osascript")
        .args(["-e", &apple_script])
        .output()
        .map_err(|e| format!("Failed to run uninstall script: {}", e))?;
    
    if output.status.success() {
        println!("Service uninstalled successfully");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Check if user cancelled the authorization
        if stderr.contains("User canceled") || stderr.contains("-128") {
            return Err("Authorization cancelled by user".to_string());
        }
        Err(format!("Uninstall failed: {}", stderr))
    }
}

#[cfg(not(target_os = "macos"))]
pub async fn install_service(_app: &tauri::AppHandle) -> Result<(), String> {
    Err("Service installation is only supported on macOS".to_string())
}

#[cfg(not(target_os = "macos"))]
pub async fn uninstall_service(_app: &tauri::AppHandle) -> Result<(), String> {
    Err("Service uninstallation is only supported on macOS".to_string())
}
