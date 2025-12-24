// Windows-specific implementations
// 
// Provides Windows-specific functionality including:
// - System proxy settings via Registry
// - Process and port management
// - Future: TUN mode support via WinTun

#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

/// Set system proxy on Windows
#[cfg(target_os = "windows")]
pub async fn set_system_proxy_windows(
    enabled: bool,
    http_port: u16,
    socks_port: u16,
) -> Result<(), String> {
    use std::ptr;
    
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let internet_settings = hkcu
        .open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Internet Settings",
            KEY_WRITE,
        )
        .map_err(|e| format!("Failed to open registry key: {}", e))?;

    if enabled {
        // Set ProxyEnable to 1
        internet_settings
            .set_value("ProxyEnable", &1u32)
            .map_err(|e| format!("Failed to enable proxy: {}", e))?;

        // Set ProxyServer (HTTP and HTTPS use the same port)
        let proxy_server = format!("http=127.0.0.1:{};https=127.0.0.1:{};socks=127.0.0.1:{}", 
            http_port, http_port, socks_port);
        internet_settings
            .set_value("ProxyServer", &proxy_server)
            .map_err(|e| format!("Failed to set proxy server: {}", e))?;

        // Set ProxyOverride to bypass local addresses
        internet_settings
            .set_value("ProxyOverride", &"localhost;127.*;10.*;172.16.*;172.17.*;172.18.*;172.19.*;172.20.*;172.21.*;172.22.*;172.23.*;172.24.*;172.25.*;172.26.*;172.27.*;172.28.*;172.29.*;172.30.*;172.31.*;192.168.*;<local>")
            .map_err(|e| format!("Failed to set proxy override: {}", e))?;

        println!("Windows system proxy enabled: {}", proxy_server);
    } else {
        // Set ProxyEnable to 0
        internet_settings
            .set_value("ProxyEnable", &0u32)
            .map_err(|e| format!("Failed to disable proxy: {}", e))?;

        println!("Windows system proxy disabled");
    }

    // Notify Windows that Internet settings have changed
    unsafe {
        winapi::um::wininet::InternetSetOptionW(
            ptr::null_mut(),
            winapi::um::wininet::INTERNET_OPTION_SETTINGS_CHANGED,
            ptr::null_mut(),
            0,
        );
        
        // Refresh settings
        winapi::um::wininet::InternetSetOptionW(
            ptr::null_mut(),
            winapi::um::wininet::INTERNET_OPTION_REFRESH,
            ptr::null_mut(),
            0,
        );
    }

    Ok(())
}

/// Get current system proxy status on Windows
#[cfg(target_os = "windows")]
pub fn get_system_proxy_status_windows() -> Result<bool, String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let internet_settings = hkcu
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Internet Settings")
        .map_err(|e| format!("Failed to open registry key: {}", e))?;

    let proxy_enable: u32 = internet_settings
        .get_value("ProxyEnable")
        .unwrap_or(0);

    Ok(proxy_enable == 1)
}

/// Check if a port is in use on Windows
#[cfg(target_os = "windows")]
pub fn is_port_in_use_windows(port: u16) -> bool {
    use std::net::TcpListener;
    TcpListener::bind(format!("127.0.0.1:{}", port)).is_err()
}

/// Find process ID by port on Windows
#[cfg(target_os = "windows")]
pub fn find_pid_by_port_windows(port: u16) -> Option<u32> {
    use std::process::Command;
    
    let output = Command::new("netstat")
        .args(["-ano"])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    for line in stdout.lines() {
        // Look for lines containing the port and LISTENING state
        if line.contains(&format!(":{}", port)) && line.contains("LISTENING") {
            // netstat output format: Proto  Local Address  Foreign Address  State  PID
            // Extract the last column which is the PID
            if let Some(pid_str) = line.split_whitespace().last() {
                if let Ok(pid) = pid_str.parse::<u32>() {
                    return Some(pid);
                }
            }
        }
    }
    
    None
}

/// Kill process by PID on Windows
#[cfg(target_os = "windows")]
pub fn kill_process_windows(pid: u32) -> Result<(), String> {
    use std::process::Command;
    
    let output = Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .output()
        .map_err(|e| format!("Failed to kill process: {}", e))?;

    if output.status.success() {
        println!("Successfully killed process {}", pid);
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to kill process {}: {}", pid, stderr))
    }
}

/// Cleanup port on Windows (kill process using the port)
#[cfg(target_os = "windows")]
pub fn cleanup_port_windows(port: u16) -> Result<(), String> {
    if let Some(pid) = find_pid_by_port_windows(port) {
        println!("Found process {} using port {}, attempting to kill...", pid, port);
        kill_process_windows(pid)?;
        
        // Wait a bit for the port to be released
        std::thread::sleep(std::time::Duration::from_millis(500));
        
        // Verify port is now free
        if is_port_in_use_windows(port) {
            return Err(format!("Port {} is still in use after killing process", port));
        }
        
        println!("Port {} is now free", port);
        Ok(())
    } else {
        // Port is not in use or we couldn't find the process
        if is_port_in_use_windows(port) {
            Err(format!("Port {} is in use but couldn't find the process", port))
        } else {
            println!("Port {} is not in use", port);
            Ok(())
        }
    }
}

/// Check if a process is running by PID on Windows
#[cfg(target_os = "windows")]
pub fn is_pid_running_windows(pid: u32) -> bool {
    use std::process::Command;
    
    let output = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid), "/NH"])
        .output();
    
    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // If the process exists, tasklist will show it
        // If not, it will say "INFO: No tasks are running..."
        !stdout.contains("INFO: No tasks")
    } else {
        false
    }
}

// Stub implementations for non-Windows platforms
#[cfg(not(target_os = "windows"))]
pub async fn set_system_proxy_windows(
    _enabled: bool,
    _http_port: u16,
    _socks_port: u16,
) -> Result<(), String> {
    Err("Windows system proxy is only supported on Windows".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn get_system_proxy_status_windows() -> Result<bool, String> {
    Err("Windows system proxy is only supported on Windows".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn is_port_in_use_windows(_port: u16) -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
pub fn find_pid_by_port_windows(_port: u16) -> Option<u32> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn kill_process_windows(_pid: u32) -> Result<(), String> {
    Err("Process management is only supported on Windows".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn cleanup_port_windows(_port: u16) -> Result<(), String> {
    Err("Port cleanup is only supported on Windows".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn is_pid_running_windows(_pid: u32) -> bool {
    false
}
