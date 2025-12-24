// ========== Mode Management Helpers (macOS) ==========

#[cfg(target_os = "macos")]
async fn stop_user_mode(state: &MihomoState) -> Result<(), String> {
    println!("Stopping user mode...");
    
    // Stop child process if running
    {
        let mut process_lock = state.process.lock().map_err(|e| e.to_string())?;
        if let Some(mut child) = process_lock.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    
    // Handle root_pid if running via legacy sudo mode
    let pid = {
        if let Ok(mut pid_lock) = state.root_pid.lock() {
            pid_lock.take()
        } else {
            None
        }
    };
    
    if let Some(pid) = pid {
        let _ = Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .output();
        
        for _ in 0..20 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            if !is_pid_running(pid) {
                break;
            }
        }
        
        if is_pid_running(pid) {
            // Force kill without sudo - if we spawned it, we can kill it
            // If it was spawned by Service Mode, the service handles cleanup
            let _ = Command::new("kill").args(["-9", &pid.to_string()]).output();
        }
    }
    
    // Port cleanup
    if let Ok(port) = state.api_port.lock() {
        cleanup_port(*port);
    }
    
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    Ok(())
}

#[cfg(target_os = "macos")]
async fn stop_service_mode_silent(state: &MihomoState) -> Result<bool, String> {
    if !is_privileged_helper_valid() {
        return Ok(true); // Helper not installed, treat as already stopped
    }

    let api_port = *state.api_port.lock().map_err(|e| e.to_string())?;
    let api_secret = {
        let config_lock = state.config_path.lock().map_err(|e| e.to_string())?;
        config_lock.as_ref().and_then(|p| parse_api_secret_from_file(p))
    };

    // Try silent stop via API reload with empty config in a SEPARATE file
    // This preserves existing config.yaml and avoids password prompts.
    let stop_config = format!(
        "external-controller: 127.0.0.1:{}\nsecret: '{}'\nmode: rule\n",
        api_port,
        api_secret.as_deref().unwrap_or("")
    );

    let mut silent_success = false;

    // We write to STOP_CONFIG_PATH, not SYSTEM_CONFIG_PATH
    if let Ok(_) = std::fs::write(STOP_CONFIG_PATH, stop_config) {
        let client = reqwest::Client::new();
        let reload_url = format!("http://127.0.0.1:{}/configs?force=true", api_port);
        let mut req = client.put(&reload_url);
        if let Some(s) = &api_secret {
            req = req.header("Authorization", format!("Bearer {}", s));
        }
        let payload = serde_json::json!({ "path": STOP_CONFIG_PATH });

        // Short timeout for reload
        let req = req.timeout(std::time::Duration::from_millis(1500));

        if let Ok(resp) = req.json(&payload).send().await {
            if resp.status().is_success() {
                silent_success = true;
                println!("Service mode stopped silently (idling on stop.yaml)");
            }
        }
    }

    if silent_success {
        Ok(true)
    } else {
        // Check if actually running. If it's already down/unresponsive, treat as success.
        if is_port_in_use(api_port) || is_pid_running(find_mihomo_pid_by_port(api_port).unwrap_or(0)) {
            println!("Service mode silent stop failed and core still active.");
            Ok(false)
        } else {
            println!("Service mode appears already stopped or unresponsive.");
            Ok(true)
        }
    }
}

#[cfg(target_os = "macos")]
async fn stop_service_mode(state: &MihomoState) -> Result<(), String> {
    println!("Stopping service mode...");

    let api_port = *state.api_port.lock().map_err(|e| e.to_string())?;

    let silent_success = stop_service_mode_silent(state).await?;

    if !silent_success {
        // Fallback: Check if actually running before trying launchctl
        if is_port_in_use(api_port) || is_pid_running(find_mihomo_pid_by_port(api_port).unwrap_or(0)) {
            println!("Silent stop failed and core still active, using launchctl bootout...");
            // Try without sudo first - if the service was loaded by root, this may fail
            // but that's OK, the service will be stopped on next restart anyway
            let _ = Command::new("launchctl")
                .args(["bootout", &format!("system/{}", SERVICE_LABEL)])
                .output();
        } else {
            println!("Service mode appears already stopped or unresponsive.");
        }
    }

    cleanup_port(api_port);
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    Ok(())
}

/// Cleanup on app exit for Service Mode
/// NOTE: Service Mode is designed to keep mihomo running as a system service even after app exits.
/// This function only performs cleanup tasks like disabling TUN (to restore DNS) but does NOT stop the service.
/// To stop the service, user should explicitly use Settings -> Uninstall Service Mode.
#[cfg(target_os = "macos")]
pub async fn force_stop_service_mode_on_exit(state: &MihomoState) -> Result<(), String> {
    if !is_privileged_helper_valid() {
        return Ok(()); // Service mode not installed
    }
    
    println!("App Exit: Service Mode cleanup (service will continue running)...");
    
    // Check if TUN is enabled and disable it to restore DNS
    // This prevents DNS issues after app exit
    let api_port = *state.api_port.lock().map_err(|e| e.to_string())?;
    let api_host = state.api_host.lock().map_err(|e| e.to_string())?.clone();
    
    // Try to disable TUN via API (this restores system DNS)
    let client = reqwest::Client::new();
    let url = format!("http://{}:{}/configs", api_host, api_port);
    let _ = client.patch(&url)
        .json(&serde_json::json!({"tun": {"enable": false}}))
        .send()
        .await;
    
    println!("App Exit: TUN disabled (if was enabled), DNS restored.");
    println!("App Exit: Service Mode will continue running in background.");
    println!("App Exit: Use Settings -> Uninstall Service Mode to fully stop the service.");
    
    Ok(())
}

#[cfg(target_os = "macos")]
async fn ensure_user_mode_running(
    state: State<'_, MihomoState>,
    config_path: PathBuf,
) -> Result<CoreStatus, String> {
    // First ensure service mode is stopped
    stop_service_mode(state.inner()).await?;
    
    // Then start user mode
    println!("Starting user mode...");
    let options = StartOptions {
        config_path: Some(config_path.to_string_lossy().to_string()),
        external_controller: None,
        use_root: None,
        mode: Some(CoreMode::User),
    };
    
    start_core_inner(state, Some(options)).await
}

#[cfg(target_os = "macos")]
#[allow(dead_code)]
async fn ensure_service_mode_running(
    app: tauri::AppHandle,
    state: State<'_, MihomoState>,
    config_path: PathBuf,
) -> Result<CoreStatus, String> {
    // First ensure user mode is stopped
    stop_user_mode(state.inner()).await?;
    
    // Ensure helper is installed
    if !is_privileged_helper_valid() {
        println!("Service mode requested but helper not installed, installing...");
        install_privileged_helper(app, state.clone()).await?;
    }
    
    // Start service mode
    println!("Starting service mode...");
    let options = StartOptions {
        config_path: Some(config_path.to_string_lossy().to_string()),
        external_controller: None,
        use_root: None,
        mode: Some(CoreMode::Service),
    };
    
    start_core_inner(state, Some(options)).await
}

/// Start the Mihomo core
#[tauri::command]
pub async fn start_core(
    app: tauri::AppHandle,
    state: State<'_, MihomoState>,
    options: Option<StartOptions>,
) -> Result<CoreStatus, String> {
    let res = start_core_inner(state.clone(), options).await?;

    // Wait and check if it survived
    if let Err(e) = verify_survived(state.inner()).await {
        // Clean up locks if it died
        if let Ok(mut process_lock) = state.process.lock() {
            *process_lock = None;
        }
        #[cfg(target_os = "macos")]
        if let Ok(mut pid_lock) = state.root_pid.lock() {
            *pid_lock = None;
        }
        let _ = app.emit("core-started", CoreStartedEvent { success: false, message: Some(e.clone()) });
        return Err(e);
    }

    // Emit success event
    let _ = app.emit("core-started", CoreStartedEvent { success: true, message: None });
    
    Ok(res)
}

#[cfg(target_os = "macos")]
async fn start_service_mode(
    state: State<'_, MihomoState>,
    config_path: PathBuf,
) -> Result<CoreStatus, String> {
    // 1. Read current system config to find running port/secret
    let system_config = PathBuf::from(SYSTEM_CONFIG_PATH);
    // Best effort to find old port
    let old_port = parse_external_controller_from_file(&system_config)
        .map(|(_, p)| p)
        .unwrap_or(9090);
    let old_secret = parse_api_secret_from_file(&system_config);

    // 2. Apply user overrides to the config before writing to system path
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let final_content = match serde_yaml::from_str::<serde_yaml::Value>(&content) {
        Ok(mut yaml) => {
            let overrides = crate::user_overrides::load_overrides();
            if let Err(e) = crate::user_overrides::apply_overrides_to_yaml(&mut yaml, &overrides) {
                eprintln!("Warning: Failed to apply user overrides to Service Mode config: {}", e);
                content.clone()
            } else {
                match serde_yaml::to_string(&yaml) {
                    Ok(modified) => {
                        println!("Service Mode: Applied user overrides to config");
                        modified
                    }
                    Err(e) => {
                        eprintln!("Failed to serialize modified config: {}", e);
                        content.clone()
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to parse config YAML: {}", e);
            content.clone()
        }
    };
    
    // We expect the file to be writable by user (chown user:staff was done during install)
    std::fs::write(&system_config, &final_content)
        .map_err(|e| format!("Failed to write system config: {}", e))?;

    // 3. Parse NEW config to get what we expect (from system_config, not the original config_path!)
    let (new_host, new_port) = parse_external_controller_from_file(&system_config)
        .unwrap_or(("127.0.0.1".to_string(), 9090));
    let new_secret = parse_api_secret_from_file(&system_config);

    // 4. Build API Client and Resume/Reload
    let client = reqwest::Client::new();
    let reload_url = format!("http://127.0.0.1:{}/configs?force=true", old_port);
    let mut req = client.put(&reload_url);
    if let Some(s) = &old_secret {
        req = req.header("Authorization", format!("Bearer {}", s));
    }
    let payload = serde_json::json!({
        "path": SYSTEM_CONFIG_PATH
    });

    println!("Service Mode: Reloading config via API at {}", reload_url);
    let resp = req.json(&payload).send().await;

    // If reload fails (e.g. service crashed, or port changed and we missed it), fallback to restart
    let mut reloaded = false;
    if let Ok(r) = resp {
        if r.status().is_success() {
            reloaded = true;
        } else {
            println!("Service Mode: API reload failed with status: {}", r.status());
        }
    } else {
        println!("Service Mode: API request failed");
    }

    if !reloaded {
        println!("Service Mode: Attempting to restart service via launchctl...");
        if let Err(err) = enable_service_launchdaemon().await {
            println!("Service Mode: Failed to restart service: {}", err);
        } else {
            tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
        }
    }

    // 5. Update State
    {
        let mut host_lock = state.api_host.lock().map_err(|e| e.to_string())?;
        *host_lock = new_host.clone();
        let mut port_lock = state.api_port.lock().map_err(|e| e.to_string())?;
        *port_lock = new_port;
        
        // IMPORTANT: Update config_path to point to system config
        // This ensures resolve_config_path() returns the correct path for Service Mode
        let mut config_lock = state.config_path.lock().map_err(|e| e.to_string())?;
        *config_lock = Some(PathBuf::from(SYSTEM_CONFIG_PATH));
        
        // We consider the process "running" even if we don't own the child process
        let mut process_lock = state.process.lock().map_err(|e| e.to_string())?;
        *process_lock = None; 
        
        // Also clear root_pid since we don't track it
        let mut pid_lock = state.root_pid.lock().map_err(|e| e.to_string())?;
        *pid_lock = None;

        // Reset stopped flag
        if let Ok(mut stopped) = state.manually_stopped.lock() {
            *stopped = false;
        }
    }

    Ok(CoreStatus {
        running: true,
        version: None, 
        config_path: Some(SYSTEM_CONFIG_PATH.to_string()),
        api_host: new_host.clone(),
        api_port: new_port,
        api_endpoint: format!("http://{}:{}", new_host, new_port),
        api_secret: new_secret,
        uptime_seconds: Some(0),
        message: Some("Running in Service Mode".to_string()),
    })
}

#[allow(unreachable_code)]
async fn start_core_inner(
    state: State<'_, MihomoState>,
    options: Option<StartOptions>,
) -> Result<CoreStatus, String> {
    // Reset stopped flag at the beginning of any start operation
    if let Ok(mut stopped) = state.manually_stopped.lock() {
        *stopped = false;
    }

    // Determine config path first
    let config_path = if let Some(ref opts) = options {
        if let Some(ref path) = opts.config_path {
            PathBuf::from(path)
        } else {
            match crate::profiles::get_active_profile_path() {
                Ok(Some(path)) => PathBuf::from(path),
                _ => get_config_dir().join("config.yaml"),
            }
        }
    } else {
        match crate::profiles::get_active_profile_path() {
            Ok(Some(path)) => PathBuf::from(path),
            _ => get_config_dir().join("config.yaml"),
        }
    };

    println!("Starting core with config path: {:?}", config_path);

    // Apply User Overrides (Generic Logic)
    //
    // IMPORTANT: Do NOT write to a short-lived temp file (e.g. `*.yaml.tmp`) and delete it on return.
    // Mihomo may open the config file slightly after process spawn; deleting early causes it to fall back
    // to an auto-generated “initial config” (symptom: proxies missing; log says "Can't find config").
    //
    // We instead write a stable runtime config file under app config dir and reuse it across restarts.
    let actual_config_path = {
        let overrides = crate::user_overrides::load_overrides();
        println!("Loaded user overrides: {:?}", overrides);

        let overrides_empty = overrides.port.is_none()
            && overrides.socks_port.is_none()
            && overrides.mixed_port.is_none()
            && overrides.redir_port.is_none()
            && overrides.tproxy_port.is_none()
            && overrides.allow_lan.is_none()
            && overrides.external_controller.is_none()
            && overrides
                .tun
                .as_ref()
                .map(|tun| {
                    tun.enable.is_none()
                        && tun.stack.is_none()
                        && tun.device_id.is_none()
                        && tun.mtu.is_none()
                        && tun.strict_route.is_none()
                        && tun.auto_route.is_none()
                        && tun.auto_detect_interface.is_none()
                        && tun.dns_hijack.is_none()
                })
                .unwrap_or(true);

        if overrides_empty {
            config_path.clone()
        } else {
            match std::fs::read_to_string(&config_path) {
                Ok(content) => match serde_yaml::from_str::<serde_yaml::Value>(&content) {
                    Ok(mut yaml) => {
                        if let Err(e) = crate::user_overrides::apply_overrides_to_yaml(&mut yaml, &overrides) {
                            eprintln!("Warning: Failed to apply user overrides: {}", e);
                            config_path.clone()
                        } else {
                            match serde_yaml::to_string(&yaml) {
                                Ok(modified_content) => {
                                    let runtime_dir = get_config_dir();
                                    if let Err(e) = std::fs::create_dir_all(&runtime_dir) {
                                        eprintln!(
                                            "Failed to create runtime config dir {:?}: {}",
                                            runtime_dir, e
                                        );
                                        config_path.clone()
                                    } else {
                                        let runtime_path = runtime_dir.join("config.runtime.yaml");
                                        if let Err(e) = std::fs::write(&runtime_path, &modified_content) {
                                            eprintln!(
                                                "Failed to write runtime config {:?}: {}",
                                                runtime_path, e
                                            );
                                            config_path.clone()
                                        } else {
                                            println!(
                                                "Applied user overrides, using runtime config: {:?}",
                                                runtime_path
                                            );
                                            runtime_path
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Failed to serialize modified config: {}", e);
                                    config_path.clone()
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to parse config YAML: {}", e);
                        config_path.clone()
                    }
                },
                Err(e) => {
                    eprintln!("Failed to read config file: {}", e);
                    config_path.clone()
                }
            }
        }
    };

    // ========== macOS Dual-Mode Logic ==========
    #[cfg(target_os = "macos")]
    {
        let mut user_mode_block_error: Option<String> = None;
        // Determine target mode: explicit from options, or from desired_mode
        let target_mode = options
            .as_ref()
            .and_then(|opts| opts.mode)
            .or_else(|| {
                state.desired_mode.lock().ok().map(|m| *m)
            })
            .unwrap_or(CoreMode::User);
        
        println!("Target mode: {:?}", target_mode);
        
        // Check for transition lock
        if let Ok(mut pending) = state.pending_transition.lock() {
            if *pending {
                return Err("A mode transition is already in progress".to_string());
            }
            *pending = true;
        }
        
        let result: Option<Result<CoreStatus, String>> = match target_mode {
            CoreMode::Service => {
                // Ensure user mode is stopped first
                let _ = stop_user_mode(state.inner()).await;
                
                // Verify helper is available
                if !is_privileged_helper_valid() {
                    // Release lock before returning error
                    if let Ok(mut pending) = state.pending_transition.lock() {
                        *pending = false;
                    }
                    return Err("Service Mode is not installed. Please install it from Settings before starting Mihomo.".to_string());
                }
                
                // Start service mode
                let res = start_service_mode(state.clone(), actual_config_path.clone()).await;
                
                // Update current mode on success
                if res.is_ok() {
                    if let Ok(mut mode) = state.current_mode.lock() {
                        *mode = CoreMode::Service;
                    }
                }
                
                Some(res)
            }
            CoreMode::User => {
                // Check if Service Mode is actually running before attempting to stop it
                let service_running = is_privileged_helper_loaded();
                
                if service_running {
                    println!("Service Mode LaunchDaemon is loaded, attempting to stop...");
                    
                    // Try to silently stop Service Mode first (no password prompt)
                    let silent_stop_result = stop_service_mode_silent(state.inner()).await;
                    
                    // If silent stop failed (Service Mode still running), we need to disable
                    // the LaunchDaemon to prevent dual-core scenario. This requires admin privileges.
                    if let Ok(false) = silent_stop_result {
                        println!("Service Mode still active after silent stop, disabling LaunchDaemon...");
                        if let Err(e) = disable_service_launchdaemon().await {
                            user_mode_block_error = Some(format!(
                                "Service Mode is running and could not be disabled: {}. \
Please go to Settings and switch to User Mode, or manually stop the privileged helper.",
                                e
                            ));
                        } else {
                            // Successfully disabled, wait a bit for it to shut down
                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        }
                    }
                } else {
                    println!("Service Mode is not running, proceeding directly to User Mode startup");
                }
                
                // Continue with user mode startup below
                if let Ok(mut mode) = state.current_mode.lock() {
                    *mode = CoreMode::User;
                }
                
                // Will continue to user mode implementation below
                None
            }
        };
        
        // Release transition lock
        if let Ok(mut pending) = state.pending_transition.lock() {
            *pending = false;
        }

        // If we detected that Service Mode is still active, abort before
        // starting a user-mode core. This guarantees we never run both a
        // root-owned and a user-owned mihomo simultaneously from a single
        // "start core" action.
        if let Some(err) = user_mode_block_error {
            return Err(err);
        }
        
        // If Service mode was started, return its result
        if let Some(res) = result {
            return res;
        }
        
        // Otherwise continue to user mode implementation
    }

    // ========== User Mode / Non-macOS Standard Mode ==========
    
    // Check if already running
    if is_core_running(state.inner()) {
        if let Ok(lock) = state.process.lock() {
            if lock.is_none() {
                let api_port = *state.api_port.lock().map_err(|e| e.to_string())?;
                #[cfg(target_os = "macos")]
                cleanup_port(api_port);
            } else {
                return Err("Core is already running".to_string());
            }
        }
    }

    let mihomo_path = get_mihomo_path();
    if !mihomo_path.exists() {
        return Err(format!(
            "Mihomo binary not found at {:?}. Please download it first.",
            mihomo_path
        ));
    }

    let api_secret = parse_api_secret_from_file(&actual_config_path);

    // Determine API host and port before spawn
    let mut api_host_val = {
        state.api_host.lock().map_err(|e| e.to_string())?.clone()
    };
    let mut api_port_val = {
        *state.api_port.lock().map_err(|e| e.to_string())?
    };

    let parsed = options
        .as_ref()
        .and_then(|opts| opts.external_controller.as_ref())
        .and_then(|ec| parse_external_controller(ec))
        .or_else(|| parse_external_controller_from_file(&actual_config_path));

    if let Some((host, port)) = parsed {
        api_host_val = host;
        api_port_val = port;
    }
    let api_controller = format!("{}:{}", api_host_val, api_port_val);

    // Pre-start cleanup: ensure port is free
    cleanup_port(api_port_val);

    let log_file_path = get_logs_dir().join(format!("mihomo_{}.log", chrono::Local::now().format("%Y%m%d")));
    if let Some(parent) = log_file_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // NOTE: "use_root" options are deprecated in favor of Service Mode.
    // TUN mode now requires Service Mode, no password prompts during normal operation.
    let explicit_root = options.as_ref().and_then(|o| o.use_root);
    #[allow(unused_mut)]
    let mut use_root = explicit_root.unwrap_or(false);

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    if explicit_root.is_none() {
        let tun_enabled = read_tun_from_config(&actual_config_path).unwrap_or(false);
        println!("TUN enabled check: tun_enabled={}, actual_config_path={:?}", tun_enabled, actual_config_path);
        if tun_enabled {
            #[cfg(target_os = "macos")]
            {
                // Check if Service Mode is available
                if is_privileged_helper_valid() {
                    return Err(
                        "TUN mode requires elevated privileges. Please use Service Mode instead of User Mode, \
                        or disable TUN in your configuration. Service Mode is already installed - you can switch \
                        to it in Settings.".to_string()
                    );
                } else {
                    return Err(
                        "TUN mode requires elevated privileges. Please install and use Service Mode from Settings, \
                        or disable TUN in your configuration (set tun.enable to false).".to_string()
                    );
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                use_root = true;
            }
        }
    }

    #[cfg(target_os = "macos")]
    if use_root {
        // TUN mode requires Service Mode on macOS
        // Instead of using osascript (which prompts for password every time),
        // we redirect users to install Service Mode for a better experience
        return Err("TUN mode requires Service Mode. Please enable Service Mode in Settings first.".to_string());
    }
    
    // This block is now only for non-root User Mode on macOS
    #[cfg(target_os = "macos")]
    #[allow(clippy::if_same_then_else)]
    if false {
        // Placeholder to maintain code structure - this branch is never taken
        let api_host_val = api_host_val.clone();
        let mut port_lock = state.api_port.lock().map_err(|e| e.to_string())?;
        *port_lock = api_port_val;
        
        // Clear manually_stopped flag
        if let Ok(mut stopped) = state.manually_stopped.lock() {
            *stopped = false;
        }

        let api_endpoint = build_api_endpoint(&api_host_val, api_port_val);

        return Ok(CoreStatus {
            running: true,
            version: None,
            config_path: Some(actual_config_path.to_string_lossy().to_string()),
            api_host: api_host_val.to_string(),
            api_port: api_port_val,
            api_endpoint,
            api_secret: api_secret.clone(),
            uptime_seconds: Some(0),
            message: None,
        });
    }

    // Build command for normal (non-root) execution
    #[cfg(target_os = "linux")]
    let mut cmd = if use_root {
        let mut c = Command::new("pkexec");
        c.arg(&mihomo_path);
        c
    } else {
        Command::new(&mihomo_path)
    };

    #[cfg(not(target_os = "linux"))]
    let mut cmd = Command::new(&mihomo_path);

    cmd.arg("-d")
        .arg(actual_config_path.parent().unwrap_or(&get_config_dir()));
    cmd.arg("-f").arg(&actual_config_path);
    cmd.arg("-ext-ctl").arg(&api_controller);

    // Open in append mode
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
        .map_err(|e| format!("Failed to open log file: {}", e))?;

    let log_file_err = log_file
        .try_clone()
        .map_err(|e| format!("Failed to clone log file handle: {}", e))?;

    let child = cmd
        .stdout(std::process::Stdio::from(log_file))
        .stderr(std::process::Stdio::from(log_file_err))
        .spawn()
        .map_err(|e| format!("Failed to start Mihomo: {}", e))?;

    let mut process_lock = state.process.lock().map_err(|e| e.to_string())?;
    *process_lock = Some(child);
    #[cfg(target_os = "macos")]
    {
        if let Ok(mut pid_lock) = state.root_pid.lock() {
            *pid_lock = None;
        }
    }

    // Update config path (use the actual runtime config that Mihomo is reading)
    let mut config_lock = state.config_path.lock().map_err(|e| e.to_string())?;
    *config_lock = Some(actual_config_path.clone());

    // Update api host/port from options or config
    let mut api_host = state.api_host.lock().map_err(|e| e.to_string())?;
    let mut api_port = state.api_port.lock().map_err(|e| e.to_string())?;
    let parsed = options
        .as_ref()
        .and_then(|opts| opts.external_controller.as_ref())
        .and_then(|ec| parse_external_controller(ec))
        .or_else(|| parse_external_controller_from_file(&actual_config_path));
    if let Some((host, port)) = parsed {
        *api_host = host;
        *api_port = port;
    }
    let api_endpoint = build_api_endpoint(&api_host, *api_port);
    
    // Clear manually_stopped flag
    if let Ok(mut stopped) = state.manually_stopped.lock() {
        *stopped = false;
    }

    Ok(CoreStatus {
        running: true,
        version: None,
        config_path: Some(actual_config_path.to_string_lossy().to_string()),
        api_host: api_host.to_string(),
        api_port: *api_port,
        api_endpoint,
        api_secret,
        uptime_seconds: Some(0),
        message: None,
    })
}

/// Stop the Mihomo core
#[tauri::command]
pub async fn stop_core(app: tauri::AppHandle, state: State<'_, MihomoState>) -> Result<(), String> {
    // Safer default: when stopping the core, also disable system proxy.
    // This prevents the OS from pointing to a dead local proxy endpoint and “breaking the network”.
    let _ = set_system_proxy(app.clone(), false, None).await;

    let result = stop_core_inner(state.inner()).await;
    
    // Emit stopped event
    let _ = app.emit("core-stopped", CoreStoppedEvent { success: result.is_ok() });
    
    result
}

pub async fn stop_core_inner(state: &MihomoState) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // If we are in Service Mode, do NOT attempt to kill arbitrary PIDs.
        // Service Mode is managed by launchctl and should be stopped via API/launchctl logic.
        let is_service_mode = state
            .current_mode
            .lock()
            .ok()
            .map(|m| matches!(*m, CoreMode::Service))
            .unwrap_or(false);

        if is_service_mode {
            println!("Stopping core in Service Mode...");
            let _ = stop_service_mode(state).await;

            // Mark stopped to keep UI consistent
            if let Ok(mut stopped) = state.manually_stopped.lock() {
                *stopped = true;
            }
            return Ok(());
        }
    }

    {
        let mut process_lock = state.process.lock().map_err(|e| e.to_string())?;
        if let Some(mut child) = process_lock.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    #[cfg(target_os = "macos")]
    {
        // If we are in privileged mode, the process is already in process_lock and was killed above.
        // If we are in legacy sudo mode, we need to handle root_pid.
        let is_privileged = {
            if let Ok(lock) = state.use_privileged_mode.lock() {
                *lock
            } else {
                false
            }
        };

        if !is_privileged {
            let pid = {
                if let Ok(mut pid_lock) = state.root_pid.lock() {
                    pid_lock.take()
                } else {
                    None
                }
            };

            if let Some(pid) = pid {
                // Never kill ourselves (guard against incorrect PID recovery).
                let current_pid = std::process::id();
                if pid == current_pid {
                    println!("Refusing to kill current app PID {}", pid);
                    return Ok(());
                }

                // Try to kill with TERM signal
                let _ = Command::new("kill")
                    .arg("-TERM")
                    .arg(pid.to_string())
                    .output();

                // Wait up to 2 seconds for the process to exit
                for _ in 0..20 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    if !is_pid_running(pid) {
                        break;
                    }
                }

                // If still running, force kill (no sudo needed for processes we spawned)
                if is_pid_running(pid) {
                    let _ = Command::new("kill").args(["-9", &pid.to_string()]).output();
                }
            }
        }

        // --- SERVICE MODE STOP ---
        // If Service Mode is active (helper installed), we PREFER to stop it silently by reloading an empty config.
        // This avoids password prompt on every "Stop" or "App Exit".
        if is_privileged_helper_valid() {
            println!("Service Mode: Attempting silent stop via API reload...");
            
            let api_port = *state.api_port.lock().map_err(|e| e.to_string())?;
            let api_secret = {
                let config_lock = state.config_path.lock().map_err(|e| e.to_string())?;
                config_lock.as_ref().and_then(|p| parse_api_secret_from_file(p))
            };

            // Prepare a "Stop" config that has no proxy ports
            let stop_config = format!(
                "external-controller: 127.0.0.1:{}\nsecret: '{}'\nmode: rule\n",
                api_port,
                api_secret.as_deref().unwrap_or("")
            );

            let mut silent_success = false;
            if let Ok(_) = std::fs::write(STOP_CONFIG_PATH, stop_config) {
                // Try to reload via API
                let client = reqwest::Client::new();
                let reload_url = format!("http://127.0.0.1:{}/configs?force=true", api_port);
                let mut req = client.put(&reload_url);
                if let Some(s) = &api_secret {
                    req = req.header("Authorization", format!("Bearer {}", s));
                }
                let payload = serde_json::json!({ "path": STOP_CONFIG_PATH });
                
                // Short timeout
                let req = req.timeout(std::time::Duration::from_millis(1500));
                
                if let Ok(resp) = req.json(&payload).send().await {
                    if resp.status().is_success() {
                        println!("Service Mode: Silent stop success (idling).");
                        silent_success = true;
                    }
                }
            }

            if silent_success {
                // Set flag to ensure is_core_running returns false immediately
                if let Ok(mut stopped) = state.manually_stopped.lock() {
                    *stopped = true;
                }
            } else {
                // Fallback: Check if active before trying launchctl
                if is_port_in_use(api_port) {
                    println!("Service Mode: Silent stop failed, trying launchctl bootout without sudo...");
                    // Try without sudo - if it fails, that's OK, service will be stopped next restart
                    let _ = Command::new("launchctl")
                        .args(["bootout", &format!("system/{}", SERVICE_LABEL)])
                        .output();
                } else {
                    println!("Service Mode: Silent stop failed but port is closed, assuming stopped.");
                }
            }
        }

        // Final port cleanup (handles orphans regardless of mode)
        if let Ok(port) = state.api_port.lock() {
            cleanup_port(*port);
        }
        
        // Wait a bit to ensure port is released by the OS
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    // Set manually stopped flag at the very end to ensure UI sync (All Platforms)
    if let Ok(mut stopped) = state.manually_stopped.lock() {
        *stopped = true;
    }

    Ok(())
}

#[cfg(target_os = "macos")]
const SERVICE_PLIST_PATH: &str = "/Library/LaunchDaemons/com.aqiu.service.plist";
#[cfg(target_os = "macos")]
#[allow(dead_code)]
const SYSTEM_DIR: &str = "/Library/Application Support/aqiu";
#[cfg(target_os = "macos")]
#[allow(dead_code)]
const SYSTEM_BINARY_PATH: &str = "/Library/Application Support/aqiu/aqiu-mihomo";
#[cfg(target_os = "macos")]
const SYSTEM_CONFIG_PATH: &str = "/Library/Application Support/aqiu/config.yaml";
#[cfg(target_os = "macos")]
const STOP_CONFIG_PATH: &str = "/Library/Application Support/aqiu/stop.yaml";
#[cfg(target_os = "macos")]
const SERVICE_LABEL: &str = "com.aqiu.service";

#[cfg(target_os = "macos")]
const SERVICE_PLIST_CONTENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.aqiu.service</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Library/Application Support/aqiu/aqiu-mihomo</string>
        <string>-d</string>
        <string>/Library/Application Support/aqiu</string>
        <string>-f</string>
        <string>/Library/Application Support/aqiu/config.yaml</string>
    </array>
    <key>KeepAlive</key>
    <true/>
    <key>RunAtLoad</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/Library/Application Support/aqiu/service.log</string>
    <key>StandardErrorPath</key>
    <string>/Library/Application Support/aqiu/service.log</string>
</dict>
</plist>
"#;

#[cfg(target_os = "macos")]
#[allow(dead_code)]
fn get_privileged_helper_path() -> PathBuf {
    PathBuf::from(SERVICE_PLIST_PATH) // Changed semantics: now checks plist existence
}

#[cfg(target_os = "macos")]
fn is_privileged_helper_valid() -> bool {
    std::path::Path::new(SERVICE_PLIST_PATH).exists()
}

#[cfg(target_os = "macos")]
fn is_privileged_helper_loaded() -> bool {
    if !is_privileged_helper_valid() {
        return false;
    }

    if let Ok(output) = Command::new("launchctl")
        .arg("print")
        .arg(format!("system/{}", SERVICE_LABEL))
        .output()
    {
        output.status.success()
    } else {
        false
    }
}

#[cfg(target_os = "macos")]
async fn disable_service_launchdaemon() -> Result<(), String> {
    if !is_privileged_helper_valid() {
        return Ok(());
    }

    // Try without sudo first - launchctl commands may work for user-installed services
    let _ = Command::new("launchctl")
        .args(["bootout", &format!("system/{}", SERVICE_LABEL)])
        .output();
    
    let _ = Command::new("launchctl")
        .args(["disable", &format!("system/{}", SERVICE_LABEL)])
        .output();

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    Ok(())
}

#[cfg(target_os = "macos")]
async fn enable_service_launchdaemon() -> Result<(), String> {
    if !is_privileged_helper_valid() {
        return Err("Service Mode helper is not installed.".to_string());
    }

    // Try to enable and bootstrap the service
    // Note: These commands may fail without sudo, but often succeed if the service was previously installed
    let _ = Command::new("launchctl")
        .args(["enable", &format!("system/{}", SERVICE_LABEL)])
        .output();
    
    let output = Command::new("launchctl")
        .args(["bootstrap", "system", SERVICE_PLIST_PATH])
        .output();
    
    if output.is_err() || !output.as_ref().unwrap().status.success() {
        // Fallback to kickstart
        let _ = Command::new("launchctl")
            .args(["kickstart", "-k", &format!("system/{}", SERVICE_LABEL)])
            .output();
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    Ok(())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn get_privileged_helper_status() -> Result<bool, String> {
    Ok(is_privileged_helper_loaded())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn install_privileged_helper(
    app: tauri::AppHandle,
    state: tauri::State<'_, MihomoState>,
) -> Result<(), String> {
    use tauri::Manager;
    use std::process::Command as StdCommand;
    
    // 0. Stop any existing local process first to avoid duplicates
    let _ = stop_core_inner(state.inner()).await;

    let mihomo_path = get_mihomo_path();
    if !mihomo_path.exists() {
        return Err("Mihomo binary not found. Please download it first.".to_string());
    }

    let user = Command::new("id")
        .arg("-un")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "root".to_string());

    // Prepare a temporary plist file
    let temp_plist = std::env::temp_dir().join("com.aqiu.service.plist");
    std::fs::write(&temp_plist, SERVICE_PLIST_CONTENT)
        .map_err(|e| format!("Failed to create temp plist: {}", e))?;

    // Find the install script
    let resource_dir = app.path().resource_dir()
        .map_err(|e| format!("Cannot get resource directory: {}", e))?;
    
    let install_script = resource_dir.join("install-service.sh");
    
    // Fallback to dev paths if not found
    let install_script = if install_script.exists() {
        install_script
    } else {
        // Try dev paths
        let dev_paths = [
            std::path::PathBuf::from("resources/install-service.sh"),
            std::path::PathBuf::from("../resources/install-service.sh"),
        ];
        dev_paths.into_iter().find(|p| p.exists())
            .ok_or("Install script not found")?
    };

    println!("Installing Service Mode with script: {:?}", install_script);

    // Use osascript with AppleScript to show native macOS authorization dialog
    // This is the ONLY place that requires admin password (one-time setup)
    // Use /bin/bash to execute script since it may not have +x permission
    let install_shell = install_script.to_string_lossy();
    let mihomo_shell = mihomo_path.to_string_lossy();
    let plist_shell = temp_plist.to_string_lossy();
    let prompt = "AQiu needs administrator privileges to install Service Mode.";
    let apple_script = format!(
        r#"do shell script "/bin/bash '{}' '{}' '{}' '{}'" with administrator privileges with prompt "{}""#,
        install_shell, mihomo_shell, plist_shell, user, prompt
    );
    
    let output = StdCommand::new("osascript")
        .args(["-e", &apple_script])
        .output()
        .map_err(|e| format!("Failed to run install script: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Check if user cancelled the authorization
        if stderr.contains("User canceled") || stderr.contains("-128") {
            return Err("Authorization cancelled by user".to_string());
        }
        return Err(format!("Installation failed: {}", stderr));
    }

    // Wait a moment for service to start
    tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;

    // Verify service is running
    // 1. Check if responding on ANY common port (7890 or 9090)
    if is_port_in_use(7890) || is_port_in_use(9090) || is_port_in_use(29090) {
        return Ok(());
    }
    
    // 2. Double check with launchctl
    if let Ok(true) = get_privileged_helper_status().await {
        Ok(())
    } else {
        Err("Service installed but failed to start. Check logs at /Library/Application Support/aqiu/service.log".to_string())
    }
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn uninstall_privileged_helper(
    app: tauri::AppHandle,
    state: tauri::State<'_, MihomoState>,
) -> Result<(), String> {
    use tauri::Manager;
    use std::process::Command as StdCommand;
    
    println!("Uninstalling privileged helper...");
    
    // Check if core was running before uninstall
    let was_running = is_core_running(state.inner());
    let active_config = if was_running {
        state.config_path.lock()
            .ok()
            .and_then(|lock| lock.clone())
    } else {
        None
    };
    
    // Stop service mode
    stop_service_mode(state.inner()).await?;
    
    // Find the uninstall script
    let resource_dir = app.path().resource_dir()
        .map_err(|e| format!("Cannot get resource directory: {}", e))?;
    
    let uninstall_script = resource_dir.join("uninstall-service.sh");
    
    // Fallback to dev paths if not found
    let uninstall_script = if uninstall_script.exists() {
        uninstall_script
    } else {
        let dev_paths = [
            std::path::PathBuf::from("resources/uninstall-service.sh"),
            std::path::PathBuf::from("../resources/uninstall-service.sh"),
        ];
        dev_paths.into_iter().find(|p| p.exists())
            .ok_or("Uninstall script not found")?
    };

    println!("Uninstalling Service Mode with script: {:?}", uninstall_script);

    // Use osascript with AppleScript to show native macOS authorization dialog
    // This is the ONLY place that requires admin password for uninstall
    // Use /bin/bash to execute script since it may not have +x permission
    let uninstall_shell = uninstall_script.to_string_lossy();
    let prompt = "AQiu needs administrator privileges to uninstall Service Mode.";
    let apple_script = format!(
        r#"do shell script "/bin/bash '{}'" with administrator privileges with prompt "{}""#,
        uninstall_shell, prompt
    );
    
    let output = StdCommand::new("osascript")
        .args(["-e", &apple_script])
        .output()
        .map_err(|e| format!("Failed to run uninstall script: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Check if user cancelled the authorization
        if stderr.contains("User canceled") || stderr.contains("-128") {
            return Err("Authorization cancelled by user".to_string());
        }
        return Err(format!("Uninstallation failed: {}", stderr));
    }
    
    println!("Privileged helper uninstalled successfully");
    
    // Update desired mode to User
    if let Ok(mut desired) = state.desired_mode.lock() {
        *desired = CoreMode::User;
    }
    
    // If core was running, automatically switch to user mode
    if was_running {
        println!("Core was running, switching to user mode...");
        
        let config_to_use = active_config
            .or_else(|| {
                crate::profiles::get_active_profile_path()
                    .ok()
                    .flatten()
                    .map(PathBuf::from)
            })
            .unwrap_or_else(|| get_config_dir().join("config.yaml"));
        
        match ensure_user_mode_running(state.clone(), config_to_use).await {
            Ok(_) => {
                println!("Successfully switched to user mode");
            }
            Err(e) => {
                eprintln!("Failed to start user mode after uninstall: {}", e);
                // Don't fail the uninstall, just log the error
            }
        }
    } else {
        // Reset stop flag so status check works normally
        if let Ok(mut stopped) = state.manually_stopped.lock() {
            *stopped = false;
        }
    }

    Ok(())
}


/// Restart the Mihomo core
#[tauri::command]
pub async fn restart_core(app: tauri::AppHandle, state: State<'_, MihomoState>) -> Result<CoreStatus, String> {
    // Stop
    stop_core(app.clone(), state.clone()).await?;

    // Wait a bit to ensure resources are released
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Start without explicit options to allow auto-detection of the current active profile
    start_core(app, state, None).await
}

/// Detect and recover orphaned core process on app startup.
/// This handles the case where the app crashed but mihomo core is still running.
#[tauri::command]
pub async fn recover_orphaned_core(state: State<'_, MihomoState>) -> Result<bool, String> {
    println!("Checking for orphaned core process...");
    
    let api_port = *state.api_port.lock().map_err(|e| e.to_string())?;
    
    // Check if something is listening on the API port
    if !is_port_in_use(api_port) {
        println!("No process listening on port {}, no recovery needed", api_port);
        return Ok(false);
    }
    
    // Try to find the PID
    #[cfg(target_os = "macos")]
    let orphan_pid = find_mihomo_pid_by_port(api_port);
    #[cfg(not(target_os = "macos"))]
    let orphan_pid: Option<u32> = None;
    
    if orphan_pid.is_none() {
        println!("Port {} is in use but couldn't identify the process", api_port);
        return Ok(false);
    }
    
    let pid = orphan_pid.unwrap();
    println!("Found orphaned core process with PID {} on port {}", pid, api_port);
    
    // Try to verify it's actually mihomo by calling the API
    let api_host = state.api_host.lock().map_err(|e| e.to_string())?.clone();
    let version = get_version_from_api(&api_host, api_port).await;
    
    if version.is_err() {
        println!("Process on port {} is not responding to mihomo API, not recovering", api_port);
        return Ok(false);
    }
    
    println!("Verified orphaned process is mihomo (version: {:?}), recovering state...", version);
    
    // Update state to reflect the running core
    #[cfg(target_os = "macos")]
    {
        // Check if it's Service Mode (LaunchDaemon) or User Mode
        if is_privileged_helper_loaded() {
            println!("Detected Service Mode LaunchDaemon, updating state...");
            if let Ok(mut mode) = state.current_mode.lock() {
                *mode = CoreMode::Service;
            }
            if let Ok(mut desired) = state.desired_mode.lock() {
                *desired = CoreMode::Service;
            }
        } else {
            println!("Detected User Mode orphaned process, caching PID...");
            if let Ok(mut pid_lock) = state.root_pid.lock() {
                *pid_lock = Some(pid);
            }
            if let Ok(mut mode) = state.current_mode.lock() {
                *mode = CoreMode::User;
            }
        }
    }
    
    // Clear manually_stopped flag
    if let Ok(mut stopped) = state.manually_stopped.lock() {
        *stopped = false;
    }
    
    // Try to find config path from the running process
    #[cfg(target_os = "macos")]
    {
        // For Service Mode, use system config path
        if is_privileged_helper_loaded() {
            if let Ok(mut config_lock) = state.config_path.lock() {
                *config_lock = Some(PathBuf::from(SYSTEM_CONFIG_PATH));
            }
        }
    }
    
    println!("Successfully recovered orphaned core process (PID {})", pid);
    Ok(true)
}

/// Get core status
#[tauri::command]
pub async fn get_core_status(state: State<'_, MihomoState>) -> Result<CoreStatus, String> {
    // Wrap the entire operation in a timeout to prevent hanging
    let timeout_duration = std::time::Duration::from_secs(5);
    
    tokio::time::timeout(timeout_duration, async {
        let running = is_core_running(state.inner());

        // Extract values from locks and drop them immediately
        let (config_path_str, api_host, api_port) = {
            let config_lock = state.config_path.lock().map_err(|e| e.to_string())?;
            let api_host = state.api_host.lock().map_err(|e| e.to_string())?.clone();
            let api_port = *state.api_port.lock().map_err(|e| e.to_string())?;

            let config_path_str = config_lock
                .as_ref()
                .map(|p| p.to_string_lossy().to_string());

            (config_path_str, api_host, api_port)
        }; // Locks are dropped here
        let api_secret = config_path_str
            .as_ref()
            .and_then(|path| parse_api_secret_from_file(&PathBuf::from(path)));

        // Try to get version from API if running (no locks held now)
        let version = if running {
            get_version_from_api(&api_host, api_port).await.ok()
        } else {
            None
        };

        let message = None;

        Ok(CoreStatus {
            running,
            version,
            config_path: config_path_str,
            api_host: api_host.clone(),
            api_port,
            api_endpoint: build_api_endpoint(&api_host, api_port),
            api_secret,
            uptime_seconds: None, // TODO: Track actual uptime
            message,
        })
    })
    .await
    .map_err(|_| "get_core_status timed out".to_string())?
}

/// Get version from Mihomo API
async fn get_version_from_api(host: &str, port: u16) -> Result<String, String> {
    let url = format!("http://{}:{}/version", host, port);

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_millis(500))
        .timeout(std::time::Duration::from_secs(1))
        .build()
        .map_err(|e| e.to_string())?;
    
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    #[derive(Deserialize)]
    struct VersionResponse {
        version: String,
    }

    let version_resp: VersionResponse = response.json().await.map_err(|e| e.to_string())?;
    Ok(version_resp.version)
}

/// Set system proxy (cross-platform)
#[tauri::command]
pub async fn set_system_proxy(app: tauri::AppHandle, enable: bool, port: Option<u16>) -> Result<(), String> {
    let proxy_port = port.unwrap_or(7890);
    #[cfg(target_os = "windows")]
    let proxy_server = format!("127.0.0.1:{}", proxy_port);

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        if enable {
            // Enable proxy
            Command::new("reg")
                .args([
                    "add",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
                    "/v",
                    "ProxyEnable",
                    "/t",
                    "REG_DWORD",
                    "/d",
                    "1",
                    "/f",
                ])
                .output()
                .map_err(|e| e.to_string())?;

            // Set proxy server
            Command::new("reg")
                .args([
                    "add",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
                    "/v",
                    "ProxyServer",
                    "/t",
                    "REG_SZ",
                    "/d",
                    &proxy_server,
                    "/f",
                ])
                .output()
                .map_err(|e| e.to_string())?;
        } else {
            // Disable proxy
            Command::new("reg")
                .args([
                    "add",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
                    "/v",
                    "ProxyEnable",
                    "/t",
                    "REG_DWORD",
                    "/d",
                    "0",
                    "/f",
                ])
                .output()
                .map_err(|e| e.to_string())?;
        }

        let _ = app.emit("system-proxy-changed", SystemProxyChangedEvent { enabled: enable });
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        // Get list of network services
        let services_output = Command::new("networksetup")
            .args(["-listallnetworkservices"])
            .output()
            .map_err(|e| e.to_string())?;

        let services_str = String::from_utf8_lossy(&services_output.stdout);
        let services: Vec<&str> = services_str
            .lines()
            .skip(1) // Skip header line
            .filter(|s| !s.starts_with('*')) // Skip disabled services
            .collect();

        for service in services {
            if enable {
                // Set HTTP proxy
                let _ = Command::new("networksetup")
                    .args([
                        "-setwebproxy",
                        service,
                        "127.0.0.1",
                        &proxy_port.to_string(),
                    ])
                    .output();

                // Set HTTPS proxy
                let _ = Command::new("networksetup")
                    .args([
                        "-setsecurewebproxy",
                        service,
                        "127.0.0.1",
                        &proxy_port.to_string(),
                    ])
                    .output();

                // Set SOCKS proxy
                let _ = Command::new("networksetup")
                    .args([
                        "-setsocksfirewallproxy",
                        service,
                        "127.0.0.1",
                        &proxy_port.to_string(),
                    ])
                    .output();

                // Enable proxies
                let _ = Command::new("networksetup")
                    .args(["-setwebproxystate", service, "on"])
                    .output();
                let _ = Command::new("networksetup")
                    .args(["-setsecurewebproxystate", service, "on"])
                    .output();
                let _ = Command::new("networksetup")
                    .args(["-setsocksfirewallproxystate", service, "on"])
                    .output();
            } else {
                // Disable proxies
                let _ = Command::new("networksetup")
                    .args(["-setwebproxystate", service, "off"])
                    .output();
                let _ = Command::new("networksetup")
                    .args(["-setsecurewebproxystate", service, "off"])
                    .output();
                let _ = Command::new("networksetup")
                    .args(["-setsocksfirewallproxystate", service, "off"])
                    .output();
            }
        }

        let _ = app.emit("system-proxy-changed", SystemProxyChangedEvent { enabled: enable });
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;

        // Try GNOME settings first
        if enable {
            // Set proxy mode to manual
            let _ = Command::new("gsettings")
                .args(["set", "org.gnome.system.proxy", "mode", "'manual'"])
                .output();

            // Set HTTP proxy
            let _ = Command::new("gsettings")
                .args(["set", "org.gnome.system.proxy.http", "host", "'127.0.0.1'"])
                .output();
            let _ = Command::new("gsettings")
                .args([
                    "set",
                    "org.gnome.system.proxy.http",
                    "port",
                    &proxy_port.to_string(),
                ])
                .output();

            // Set HTTPS proxy
            let _ = Command::new("gsettings")
                .args(["set", "org.gnome.system.proxy.https", "host", "'127.0.0.1'"])
                .output();
            let _ = Command::new("gsettings")
                .args([
                    "set",
                    "org.gnome.system.proxy.https",
                    "port",
                    &proxy_port.to_string(),
                ])
                .output();

            // Set SOCKS proxy
            let _ = Command::new("gsettings")
                .args(["set", "org.gnome.system.proxy.socks", "host", "'127.0.0.1'"])
                .output();
            let _ = Command::new("gsettings")
                .args([
                    "set",
                    "org.gnome.system.proxy.socks",
                    "port",
                    &proxy_port.to_string(),
                ])
                .output();
        } else {
            // Disable proxy
            let _ = Command::new("gsettings")
                .args(["set", "org.gnome.system.proxy", "mode", "'none'"])
                .output();
        }

        Ok(())
    }
}

/// Download Mihomo binary (Cross-platform with progress)
#[tauri::command]
pub async fn download_core(
    window: tauri::Window,
    version: Option<String>,
) -> Result<String, String> {
    use std::env::consts::{ARCH, OS};
    use std::io::Write;
    use tauri::Emitter;

    let target_dir = get_mihomo_path()
        .parent()
        .ok_or("Invalid Mihomo binary path (no parent directory)")?
        .to_path_buf();
    std::fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

    let client = reqwest::Client::new();

    let _ = window.emit("download-progress", "Fetching release info...");

    // 1. Get Release Info
    let release_url = if let Some(v) = version {
        format!(
            "https://api.github.com/repos/MetaCubeX/mihomo/releases/tags/{}",
            v
        )
    } else {
        "https://api.github.com/repos/MetaCubeX/mihomo/releases/latest".to_string()
    };

    let resp = client
        .get(&release_url)
        .header("User-Agent", "AQiu-Proxy")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch release info: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API Error: {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    // 2. Find Asset based on OS and ARCH
    let assets = json["assets"].as_array().ok_or("No assets found")?;

    let os_keyword = match OS {
        "windows" => "windows",
        "macos" => "darwin",
        "linux" => "linux",
        _ => return Err(format!("Unsupported OS: {}", OS)),
    };

    let arch_keyword = match ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        _ => return Err(format!("Unsupported Architecture: {}", ARCH)),
    };

    let extension = if OS == "windows" { ".zip" } else { ".gz" };

    let asset = assets
        .iter()
        .find(|a| {
            let name = a["name"].as_str().unwrap_or("");
            name.contains(os_keyword)
                && name.contains(arch_keyword)
                && name.ends_with(extension)
                && !name.contains("compatible")
        })
        .ok_or(format!(
            "No binary found for {} {}",
            os_keyword, arch_keyword
        ))?;

    let download_url = asset["browser_download_url"]
        .as_str()
        .ok_or("No download URL")?;
    let total_size = asset["size"].as_u64().unwrap_or(0);

    // 3. Download with progress
    let mut response = client
        .get(download_url)
        .header("User-Agent", "AQiu-Proxy")
        .send()
        .await
        .map_err(|e| format!("Failed to download: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed: {}", response.status()));
    }

    let mut downloaded: u64 = 0;
    let mut buffer = Vec::new();

    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        downloaded += chunk.len() as u64;
        buffer.extend_from_slice(&chunk);

        if total_size > 0 {
            let progress = format!(
                "Downloading: {:.1}%",
                (downloaded as f64 / total_size as f64) * 100.0
            );
            let _ = window.emit("download-progress", progress);
        } else {
            let _ = window.emit(
                "download-progress",
                format!("Downloading: {} bytes", downloaded),
            );
        }
    }

    let _ = window.emit("download-progress", "Extracting...");

    // 4. Extract
    if OS == "windows" {
        let reader = std::io::Cursor::new(buffer);
        let mut zip =
            zip::ZipArchive::new(reader).map_err(|e| format!("Failed to open zip: {}", e))?;

        for i in 0..zip.len() {
            let mut file = zip.by_index(i).map_err(|e| e.to_string())?;
            let name = file.name();

            if name.ends_with(".exe") {
                let mut out_file = std::fs::File::create(get_mihomo_path())
                    .map_err(|e| format!("Failed to create file: {}", e))?;
                std::io::copy(&mut file, &mut out_file)
                    .map_err(|e| format!("Failed to write file: {}", e))?;
                break;
            }
        }
    } else {
        // Handle .gz for macOS/Linux
        use flate2::read::GzDecoder;
        use std::io::Read;

        let cursor = std::io::Cursor::new(buffer);
        let mut decoder = GzDecoder::new(cursor);
        let mut output_buffer = Vec::new();
        decoder
            .read_to_end(&mut output_buffer)
            .map_err(|e| format!("Failed to decompress: {}", e))?;

        let target_path = get_mihomo_path();
        let mut out_file = std::fs::File::create(&target_path)
            .map_err(|e| format!("Failed to create file: {}", e))?;
        out_file
            .write_all(&output_buffer)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&target_path)
                .map_err(|e| format!("Failed to read permissions: {}", e))?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&target_path, perms).map_err(|e| e.to_string())?;
        }
    }

    let _ = window.emit("download-progress", "Done");
    Ok("Download complete".to_string())
}

/// Import Mihomo binary from local path
#[tauri::command]
pub fn import_core_binary(path: String) -> Result<String, String> {
    let source = PathBuf::from(path);
    if !source.exists() {
        return Err("Selected file does not exist".to_string());
    }

    let target_path = get_mihomo_path();
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    std::fs::copy(&source, &target_path).map_err(|e| format!("Failed to copy binary: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&target_path)
            .map_err(|e| format!("Failed to read permissions: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&target_path, perms).map_err(|e| e.to_string())?;
    }

    Ok(target_path.to_string_lossy().to_string())
}

/// Check if core binary exists
#[tauri::command]
pub async fn check_core_exists() -> Result<bool, String> {
    let task = tokio::task::spawn_blocking(|| get_mihomo_path().exists());
    tokio::time::timeout(std::time::Duration::from_secs(2), task)
        .await
        .map_err(|_| "check_core_exists timed out".to_string())?
        .map_err(|e| e.to_string())
}

/// Download GeoIP and GeoSite database files using mihomo official API
#[tauri::command]
pub async fn download_geodata(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, MihomoState>,
) -> Result<String, String> {
    use tauri::Emitter;

    // Check if core is running
    if !is_core_running(state.inner()) {
        return Err("Core is not running. Please start the core first to update GEO database.".to_string());
    }

    let _ = window.emit("download-progress", "Updating GEO database via mihomo API...");

    // Get API credentials
    let (api_host, api_port, api_secret) = {
        let host = state.api_host.lock().map_err(|e| e.to_string())?.clone();
        let port = *state.api_port.lock().map_err(|e| e.to_string())?;
        let secret = get_api_secret_from_state(state.inner());
        (host, port, secret)
    };

    // Use mihomo's official /upgrade/geo API
    let url = format!("http://{}:{}/upgrade/geo", api_host, api_port);
    
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut req = client.post(&url);
    
    // Add authorization if secret is set
    if let Some(s) = &api_secret {
        req = req.header("Authorization", format!("Bearer {}", s));
    }
    
    // Send empty JSON body as required by the API
    req = req.json(&serde_json::json!({}));
    
    println!("Updating GEO database via API: {}", url);
    
    let response = req.send().await
        .map_err(|e| format!("Failed to send GEO update request: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("GEO update failed: {} - {}", status, error_text));
    }

    let _ = window.emit("download-progress", "GEO database updated successfully");
    
    
    // Clear the progress message after 3 seconds
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        let _ = window.emit("download-progress", "");
    });
    
    Ok("GEO database updated successfully via mihomo API".to_string())
}

/// Get paths for the application
#[tauri::command]
pub fn get_app_paths() -> Result<serde_json::Value, String> {
    let mihomo_path = get_mihomo_path();
    let config_dir = get_config_dir();

    Ok(serde_json::json!({
        "mihomo_binary": mihomo_path.to_string_lossy(),
        "config_dir": config_dir.to_string_lossy(),
        "data_dir": dirs::data_local_dir().map(|p| p.join("aqiu").to_string_lossy().to_string()),
    }))
}

/// Download a profile/config from URL (deprecated, use profiles::update_profile_from_url instead)
/// This function is kept for backward compatibility but now delegates to the profiles system
#[tauri::command]
pub async fn download_profile(url: String) -> Result<String, String> {
    use crate::profiles;
    
    // Create a new profile with the URL
    let name = format!("Imported-{}", chrono::Local::now().format("%Y%m%d-%H%M%S"));
    let profile = profiles::create_profile(name.clone(), Some(url.clone()))?;
    
    // Update the profile from URL
    profiles::update_profile_from_url(profile.id.clone()).await?;
    
    // Set it as active
    profiles::set_active_profile(profile.id.clone())?;
    
    Ok(profile.file_path)
}

/// Get current system proxy status (cross-platform)
#[tauri::command]
pub fn get_system_proxy_status() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        let output = Command::new("reg")
            .args([
                "query",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
                "/v",
                "ProxyEnable",
            ])
            .output()
            .map_err(|e| e.to_string())?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.contains("0x1"))
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        let services_output = Command::new("networksetup")
            .args(["-listallnetworkservices"])
            .output()
            .map_err(|e| e.to_string())?;

        let services_str = String::from_utf8_lossy(&services_output.stdout);
        let services: Vec<&str> = services_str
            .lines()
            .skip(1)
            .filter(|s| !s.starts_with('*'))
            .collect();

        for service in services {
            let output = Command::new("networksetup")
                .args(["-getwebproxy", service])
                .output()
                .map_err(|e| e.to_string())?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("Enabled: Yes") {
                return Ok(true);
            }

            let https_output = Command::new("networksetup")
                .args(["-getsecurewebproxy", service])
                .output()
                .map_err(|e| e.to_string())?;
            let https_stdout = String::from_utf8_lossy(&https_output.stdout);
            if https_stdout.contains("Enabled: Yes") {
                return Ok(true);
            }

            let socks_output = Command::new("networksetup")
                .args(["-getsocksfirewallproxy", service])
                .output()
                .map_err(|e| e.to_string())?;
            let socks_stdout = String::from_utf8_lossy(&socks_output.stdout);
            if socks_stdout.contains("Enabled: Yes") {
                return Ok(true);
            }
        }

        Ok(false)
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;

        let output = Command::new("gsettings")
            .args(["get", "org.gnome.system.proxy", "mode"])
            .output()
            .map_err(|e| e.to_string())?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.contains("manual"))
    }
}

fn describe_tun_action(enable: bool) -> &'static str {
    if enable {
        "enabling"
    } else {
        "disabling"
    }
}

