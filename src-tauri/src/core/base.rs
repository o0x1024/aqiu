use arboard;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Mutex;
use tauri::{State, Emitter};

// ========== Core Mode Definition ==========

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoreMode {
    /// User mode: runs as child process in user's session
    User,
    /// Service mode: runs as system LaunchDaemon/service with root privileges
    Service,
}

impl Default for CoreMode {
    fn default() -> Self {
        CoreMode::User
    }
}

// ========== State Management ==========

pub struct MihomoState {
    pub process: Mutex<Option<Child>>,
    pub config_path: Mutex<Option<PathBuf>>,
    pub api_host: Mutex<String>,
    pub api_port: Mutex<u16>,
    #[cfg(target_os = "macos")]
    pub root_pid: Mutex<Option<u32>>,
    #[cfg(target_os = "macos")]
    pub use_privileged_mode: Mutex<bool>,
    pub manually_stopped: Mutex<bool>,
    /// Current running mode (User or Service)
    #[cfg(target_os = "macos")]
    pub current_mode: Mutex<CoreMode>,
    /// User's desired mode preference
    #[cfg(target_os = "macos")]
    pub desired_mode: Mutex<CoreMode>,
    /// Flag to prevent concurrent mode transitions
    #[cfg(target_os = "macos")]
    pub pending_transition: Mutex<bool>,
}

impl Default for MihomoState {
    fn default() -> Self {
        Self {
            process: Mutex::new(None),
            config_path: Mutex::new(None),
            api_host: Mutex::new("127.0.0.1".to_string()),
            api_port: Mutex::new(29090),
            #[cfg(target_os = "macos")]
            root_pid: Mutex::new(None),
            #[cfg(target_os = "macos")]
            use_privileged_mode: Mutex::new(false),
            manually_stopped: Mutex::new(false),
            #[cfg(target_os = "macos")]
            current_mode: Mutex::new(CoreMode::User),
            #[cfg(target_os = "macos")]
            desired_mode: Mutex::new(CoreMode::User),
            #[cfg(target_os = "macos")]
            pending_transition: Mutex::new(false),
        }
    }
}

// ========== Data Types ==========

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoreStatus {
    pub running: bool,
    pub version: Option<String>,
    pub config_path: Option<String>,
    pub api_host: String,
    pub api_port: u16,
    pub api_endpoint: String,
    pub api_secret: Option<String>,
    pub uptime_seconds: Option<u64>,
    pub message: Option<String>,
}

// Event payloads for Tauri emit
#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(dead_code)]
pub struct CoreStateUpdate {
    pub core_status: CoreStatus,
    pub system_proxy: bool,
    pub tun_mode: bool,
    pub proxy_mode: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct CoreStartedEvent {
    pub success: bool,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct CoreStoppedEvent {
    pub success: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct TunModeChangedEvent {
    pub enabled: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct SystemProxyChangedEvent {
    pub enabled: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProxyModeChangedEvent {
    pub mode: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct CoreModeChangedEvent {
    pub mode: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartOptions {
    pub config_path: Option<String>,
    pub external_controller: Option<String>,
    pub use_root: Option<bool>,
    #[cfg(target_os = "macos")]
    pub mode: Option<CoreMode>,
}

// ========== Commands ==========

/// Get the path to the Mihomo binary
fn get_mihomo_path() -> PathBuf {
    // Check common locations
    let app_data = dirs::data_local_dir().unwrap_or_default();
    let mihomo_dir = app_data.join("aqiu").join("mihomo");

    #[cfg(target_os = "windows")]
    let binary_name = "aqiu-mihomo.exe";

    #[cfg(not(target_os = "windows"))]
    let binary_name = "aqiu-mihomo";

    let target_path = mihomo_dir.join(binary_name);
    let legacy_path = {
        #[cfg(target_os = "windows")]
        let legacy_name = "mihomo.exe";
        #[cfg(not(target_os = "windows"))]
        let legacy_name = "mihomo";
        mihomo_dir.join(legacy_name)
    };

    if !target_path.exists() && legacy_path.exists() {
        let _ = std::fs::rename(&legacy_path, &target_path);
    }

    target_path
}

/// Get default config directory
pub fn get_config_dir() -> PathBuf {
    let app_data = dirs::data_local_dir().unwrap_or_default();
    app_data.join("aqiu").join("config")
}

#[cfg(target_os = "macos")]
#[allow(dead_code)]
const PRIVILEGED_HELPER_PATH: &str = "/Library/PrivilegedHelperTools/aqiu-mihomo";


// NOTE: Previously we generated a `*.yaml.tmp` file for “user overrides” and deleted it on return.
// That caused a race where Mihomo started slightly later and failed to open the config file,
// falling back to an auto-generated “initial config” (proxies missing).
// We now write a stable runtime config under `get_config_dir()` and do not need a temp guard.


#[cfg(any(target_os = "macos", target_os = "linux"))]
fn resolve_config_path(state: &MihomoState) -> PathBuf {
    if let Ok(lock) = state.config_path.lock() {
        if let Some(path) = lock.as_ref() {
            return path.clone();
        }
    }
    get_config_dir().join("config.yaml")
}


#[cfg(any(target_os = "macos", target_os = "linux"))]
fn read_tun_from_config(config_path: &PathBuf) -> Option<bool> {
    let content = std::fs::read_to_string(config_path).ok()?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    
    // If tun section doesn't exist, return None (caller will use unwrap_or(false))
    let tun_section = yaml.get("tun")?;
    
    // Get the enable field, default to false if not present or not a bool
    Some(tun_section.get("enable")
        .and_then(|enable| enable.as_bool())
        .unwrap_or(false))
}

/// Get logs directory
fn get_logs_dir() -> PathBuf {
    let app_data = dirs::data_local_dir().unwrap_or_default();
    app_data.join("aqiu").join("logs")
}

fn normalize_api_host(host: &str) -> String {
    match host {
        "0.0.0.0" | "::" | "[::]" => "127.0.0.1".to_string(),
        _ => host.to_string(),
    }
}

#[cfg(target_os = "macos")]
fn is_pid_running(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .output()
        .map(|out| {
            // On macOS:
            // 0 (success): process exists and we have permission
            // 1 (EPERM): process exists but we don't have permission (e.g. root process)
            // others (like 3 ESRCH): process does not exist
            out.status.success() || out.status.code() == Some(1)
        })
        .unwrap_or(false)
}

fn is_port_in_use(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_err()
}

#[cfg(target_os = "macos")]
fn find_mihomo_pid_by_port(port: u16) -> Option<u32> {
    // Use lsof to find the PID LISTENING on the given port.
    // IMPORTANT: `lsof -i :PORT` includes client connections; we must filter to LISTEN,
    // otherwise we may accidentally "discover" the AQiu app PID and kill ourselves later.
    let output = Command::new("lsof")
        .args([
            "-nP",
            "-t",
            "-iTCP",
            &format!(":{}", port),
            "-sTCP:LISTEN",
        ])
        .output()
        .ok()?;

    let current_pid = std::process::id();
    let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    for line in pid_str.lines() {
        if let Ok(pid) = line.trim().parse::<u32>() {
            if pid != current_pid {
                return Some(pid);
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn cleanup_port(port: u16) {
    if !is_port_in_use(port) {
        return;
    }

    let current_pid = std::process::id();
    
    let output = Command::new("lsof")
        .args(["-t", "-i", &format!(":{}", port)])
        .output();

    if let Ok(out) = output {
        let pid_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
        for pid in pid_str.lines() {
            if let Ok(p) = pid.parse::<u32>() {
                // Don't kill ourselves! (Prevents the app from terminating itself)
                if p == current_pid {
                    continue;
                }
                
                // Try normal TERM first
                let _ = Command::new("kill")
                    .arg("-TERM")
                    .arg(p.to_string())
                    .output();

                // On macOS, if the process is owned by root (like Service Mode),
                // the normal kill will fail. We avoid prompting for password here
                // to keep the experience "silent" during normal operations.
                // The main stop_core logic handles termination if absolutely required.
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn is_core_running(state: &MihomoState) -> bool {
    // If we explicitly marked the core as stopped, trust it (prevents UI flickering during cleanup)
    if let Ok(stopped) = state.manually_stopped.lock() {
        if *stopped {
            println!("[is_core_running] manually_stopped=true, returning false");
            return false;
        }
    }

    // Check 0: Service Mode (macOS LaunchDaemon) - highest priority
    #[cfg(target_os = "macos")]
    {
        if let Ok(mode) = state.current_mode.lock() {
            if matches!(*mode, CoreMode::Service) {
                // For Service Mode, check if LaunchDaemon is loaded
                if is_privileged_helper_loaded() {
                    println!("[is_core_running] Service Mode LaunchDaemon is loaded");
                    return true;
                } else {
                    println!("[is_core_running] Service Mode is set but LaunchDaemon not loaded");
                }
            }
        }
    }

    // Check 1: Child process (User Mode 非root启动)
    if let Ok(mut process_lock) = state.process.lock() {
        if let Some(child) = process_lock.as_mut() {
            match child.try_wait() {
                Ok(None) => {
                    println!("[is_core_running] Child process is still running");
                    return true;
                }
                Ok(Some(status)) => {
                    println!("[is_core_running] Child process exited with status: {}", status);
                    // Clear the stale child
                    *process_lock = None;
                }
                Err(e) => {
                    println!("[is_core_running] Error checking child process: {}", e);
                }
            }
        }
    }
    
    // Check 2: Root PID (legacy sudo mode)
    if let Ok(pid_lock) = state.root_pid.lock() {
        if let Some(pid) = *pid_lock {
            if is_pid_running(pid) {
                println!("[is_core_running] Root PID {} is running", pid);
                return true;
            } else {
                println!("[is_core_running] Root PID {} is not running anymore", pid);
            }
        }
    }
    
    // Check 3 (FINAL FALLBACK): Port check with lsof verification
    if let Ok(port_lock) = state.api_port.lock() {
        let port = *port_lock;
        if is_port_in_use(port) {
            #[cfg(any(target_os = "macos", target_os = "linux"))]
            {
                // Only treat the core as running if we can find a LISTENing process on the port.
                // This avoids false positives from client connections (e.g. AQiu itself).
                #[cfg(target_os = "macos")]
                {
                    if let Some(pid) = find_mihomo_pid_by_port(port) {
                        println!(
                            "[is_core_running] Port {} is LISTENing by PID {}, recovering state",
                            port, pid
                        );
                        // STATE RECOVERY: cache for legacy sudo mode only (Service Mode is handled above)
                        if let Ok(mode) = state.current_mode.lock() {
                            if !matches!(*mode, CoreMode::Service) {
                                if let Ok(mut pid_lock) = state.root_pid.lock() {
                                    *pid_lock = Some(pid);
                                }
                            }
                        }
                        return true;
                    }
                }
                #[cfg(target_os = "linux")]
                {
                    // Keep the old behavior on Linux for now (LISTEN filtering differs).
                    let output = Command::new("lsof")
                        .args(["-t", "-i", &format!(":{}", port)])
                        .output();
                    if let Ok(out) = output {
                        if !out.stdout.is_empty() {
                            return true;
                        }
                    }
                }
            }
            #[cfg(target_os = "windows")]
            {
                // On Windows, is_port_in_use is usually reliable enough
                println!("[is_core_running] Port {} is in use (Windows)", port);
                return true;
            }
        } else {
            println!("[is_core_running] Port {} is not in use", port);
        }
    }
    
    println!("[is_core_running] All checks failed, returning false");
    false
}

fn parse_external_controller(value: &str) -> Option<(String, u16)> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut parts = trimmed.rsplitn(2, ':');
    let port_str = parts.next()?;
    let host_str = parts.next()?;
    let port = port_str.parse::<u16>().ok()?;
    Some((normalize_api_host(host_str), port))
}

fn parse_external_controller_from_file(path: &PathBuf) -> Option<(String, u16)> {
    let content = std::fs::read_to_string(path).ok()?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    let controller = yaml
        .get("external-controller")
        .and_then(|v| v.as_str())
        .or_else(|| yaml.get("external_controller").and_then(|v| v.as_str()))?;
    parse_external_controller(controller)
}

fn parse_api_secret_from_file(path: &PathBuf) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    yaml.get("secret")
        .and_then(|v| v.as_str())
        .or_else(|| yaml.get("authentication").and_then(|v| v.as_str()))
        .map(|value| value.to_string())
}

fn build_api_endpoint(host: &str, port: u16) -> String {
    format!("http://{}:{}", host, port)
}

/// Get API secret from state's config path
fn get_api_secret_from_state(state: &MihomoState) -> Option<String> {
    state.config_path.lock().ok()
        .and_then(|lock| lock.as_ref().and_then(|p| parse_api_secret_from_file(p)))
}

/// Build a reqwest RequestBuilder with Authorization header if secret is available
fn add_auth_header(builder: reqwest::RequestBuilder, secret: Option<&str>) -> reqwest::RequestBuilder {
    match secret {
        Some(s) if !s.is_empty() => builder.header("Authorization", format!("Bearer {}", s)),
        _ => builder,
    }
}

async fn verify_survived(state: &MihomoState) -> Result<(), String> {
    // For Service Mode, we must allow more time: LaunchDaemon start + config reload can take seconds.
    // Also, a simple “port in use” check can be racy; prefer verifying the API responds.

    #[cfg(target_os = "macos")]
    let current_mode = state.current_mode.lock().ok().map(|m| *m);
    #[cfg(not(target_os = "macos"))]
    let current_mode: Option<CoreMode> = None;

    let (poll_interval_ms, max_attempts, describe) = match current_mode {
        #[cfg(target_os = "macos")]
        Some(CoreMode::Service) => (500u64, 30u32, "Service Mode"), // up to ~15s
        _ => (500u64, 6u32, "User Mode"), // up to ~3s
    };

    // capture API endpoint once (best-effort)
    let (api_host, api_port) = {
        let host = state
            .api_host
            .lock()
            .ok()
            .map(|guard| guard.clone())
            .unwrap_or_else(|| "127.0.0.1".to_string());
        let port = state
            .api_port
            .lock()
            .ok()
            .map(|guard| *guard)
            .unwrap_or(29090);
        (host, port)
    };

    async fn api_ready(host: &str, port: u16) -> bool {
        // `/version` does not require authentication and is fast.
        let url = format!("http://{}:{}/version", host, port);
        let client = match reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_millis(300))
            .timeout(std::time::Duration::from_millis(800))
            .build()
        {
            Ok(c) => c,
            Err(_) => return false,
        };

        match client.get(url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    for attempt in 1..=max_attempts {
        tokio::time::sleep(tokio::time::Duration::from_millis(poll_interval_ms)).await;

        // Fast path: process/port check (cheap)
        if is_core_running(state) {
            // Stronger check for readiness: API must respond (especially for service mode)
            if api_ready(&api_host, api_port).await {
                println!(
                    "Core verified as running and API ready ({} attempt {}/{})",
                    describe, attempt, max_attempts
                );
                return Ok(());
            }
            println!(
                "Core seems running but API not ready yet ({} attempt {}/{})",
                describe, attempt, max_attempts
            );
            continue;
        }

        println!(
            "Core not yet running, retrying... ({} attempt {}/{})",
            describe, attempt, max_attempts
        );
    }

    Err(format!(
        "Mihomo core did not become ready in time ({}). API not responding at {}:{}.\n\
Check logs under the app logs directory, and for Service Mode check `/Library/Application Support/aqiu/service.log`.",
        describe,
        api_host,
        api_port
    ))
}

/// Compress old log files (older than 7 days)
#[allow(dead_code)]
fn compress_old_logs(logs_dir: &PathBuf) {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let now = chrono::Local::now();
    let seven_days_ago = now - chrono::Duration::days(7);

    if let Ok(entries) = std::fs::read_dir(logs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            // Only process .log files (not already compressed)
            if path.extension().and_then(|s| s.to_str()) != Some("log") {
                continue;
            }

            // Check if file is older than 7 days
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    let modified_time: chrono::DateTime<chrono::Local> = modified.into();

                    if modified_time < seven_days_ago {
                        // Compress the file
                        if let Ok(content) = std::fs::read(&path) {
                            let gz_path = path.with_extension("log.gz");

                            if let Ok(gz_file) = std::fs::File::create(&gz_path) {
                                let mut encoder = GzEncoder::new(gz_file, Compression::default());
                                if encoder.write_all(&content).is_ok() && encoder.finish().is_ok() {
                                    // Delete original file after successful compression
                                    let _ = std::fs::remove_file(&path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

