// ========== Proxy Mode ==========

/// Set proxy mode via Mihomo API
#[tauri::command]
pub async fn set_mode(app: tauri::AppHandle, state: tauri::State<'_, MihomoState>, mode: String) -> Result<(), String> {
    let (api_host, api_port, api_secret) = {
        let host = state.api_host.lock().map_err(|e| e.to_string())?.clone();
        let port = *state.api_port.lock().map_err(|e| e.to_string())?;
        let secret = get_api_secret_from_state(state.inner());
        (host, port, secret)
    };

    let url = format!("http://{}:{}/configs", api_host, api_port);

    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "mode": mode
    });

    let request = add_auth_header(
        client.patch(&url).json(&payload).timeout(std::time::Duration::from_secs(5)),
        api_secret.as_deref()
    );
    let response = request.send().await
        .map_err(|e| format!("Failed to set mode: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to set mode: {}", response.status()));
    }

    // Emit event after successful mode change
    let _ = app.emit("proxy-mode-changed", ProxyModeChangedEvent { mode: mode.clone() });

    Ok(())
}

/// Get current proxy mode from Mihomo API
#[tauri::command]
pub async fn get_mode(state: tauri::State<'_, MihomoState>) -> Result<String, String> {
    let (api_host, api_port, api_secret) = {
        let host = state.api_host.lock().map_err(|e| e.to_string())?.clone();
        let port = *state.api_port.lock().map_err(|e| e.to_string())?;
        let secret = get_api_secret_from_state(state.inner());
        (host, port, secret)
    };

    let url = format!("http://{}:{}/configs", api_host, api_port);

    let client = reqwest::Client::new();
    let request = add_auth_header(
        client.get(&url).timeout(std::time::Duration::from_secs(5)),
        api_secret.as_deref()
    );
    let response = request.send().await
        .map_err(|e| format!("Failed to get mode: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to get mode: {}", response.status()));
    }

    let config: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    Ok(config
        .get("mode")
        .and_then(|mode| mode.as_str())
        .unwrap_or("rule")
        .to_string())
}

/// Copy proxy environment variables to clipboard
#[tauri::command]
pub async fn copy_proxy_env(state: State<'_, MihomoState>) -> Result<String, String> {
    let config_path = resolve_config_path(&state);
    if !config_path.exists() {
        return Err("Config file not found".to_string());
    }

    let content = std::fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&content).map_err(|e| e.to_string())?;

    // Mihomo port hierarchy: mixed-port > (port, socks-port)
    let mixed_port = yaml.get("mixed-port").and_then(|v| v.as_u64());
    let http_port = yaml.get("port").and_then(|v| v.as_u64());
    let socks_port = yaml.get("socks-port").and_then(|v| v.as_u64());

    let effective_http = mixed_port.or(http_port).unwrap_or(27890);
    let effective_socks = mixed_port.or(socks_port).unwrap_or(27890);

    let cmd = if cfg!(target_os = "windows") {
        format!(
            "set https_proxy=http://127.0.0.1:{} & set http_proxy=http://127.0.0.1:{} & set all_proxy=socks5://127.0.0.1:{}",
            effective_http, effective_http, effective_socks
        )
    } else {
        format!(
            "export https_proxy=http://127.0.0.1:{} http_proxy=http://127.0.0.1:{} all_proxy=socks5://127.0.0.1:{}",
            effective_http, effective_http, effective_socks
        )
    };

    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(cmd.clone()).map_err(|e| e.to_string())?;

    Ok(cmd)
}

// ========== Core Mode Management (macOS) ==========

/// Get current core mode
#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn get_core_mode(state: tauri::State<'_, MihomoState>) -> Result<String, String> {
    let current = state.current_mode.lock()
        .map_err(|e| e.to_string())?;
    
    match *current {
        CoreMode::User => Ok("user".to_string()),
        CoreMode::Service => Ok("service".to_string()),
    }
}

/// Get desired core mode preference
#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn get_desired_core_mode(state: tauri::State<'_, MihomoState>) -> Result<String, String> {
    let desired = state.desired_mode.lock()
        .map_err(|e| e.to_string())?;
    
    match *desired {
        CoreMode::User => Ok("user".to_string()),
        CoreMode::Service => Ok("service".to_string()),
    }
}

/// Set desired core mode and switch if core is running
#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn set_core_mode(
    app: tauri::AppHandle,
    state: tauri::State<'_, MihomoState>,
    mode: String,
) -> Result<(), String> {
    let target_mode = match mode.as_str() {
        "user" => CoreMode::User,
        "service" => CoreMode::Service,
        _ => return Err("Invalid mode. Use 'user' or 'service'".to_string()),
    };
    
    // When switching *to user mode*, proactively disable TUN in overrides to avoid a
    // confusing “User Mode + TUN” combination on next start (which would fail on macOS).
    // Switching *to service mode* should NOT disable TUN — users often switch modes
    // specifically to enable TUN without prompts.
    if matches!(target_mode, CoreMode::User) {
        println!("Disabling TUN mode before switching to {:?} mode", target_mode);
        if let Err(e) = crate::user_overrides::persist_tun_override(false) {
            eprintln!("Warning: Failed to disable TUN during mode switch: {}", e);
            // Continue anyway, this is not critical
        }
    }

    match target_mode {
        CoreMode::User => {
            // Only disable if LaunchDaemon is actually loaded (避免不必要的密码提示)
            if is_privileged_helper_loaded() {
                println!("Service Mode LaunchDaemon is loaded, disabling it...");
                disable_service_launchdaemon()
                    .await
                    .map_err(|e| format!("Failed to disable Service Mode: {}", e))?;
            } else {
                println!("Service Mode LaunchDaemon is not loaded, no need to disable");
            }
        }
        CoreMode::Service => {
            // Only enable if LaunchDaemon is not already loaded (避免重复密码输入)
            if !is_privileged_helper_loaded() {
                enable_service_launchdaemon()
                    .await
                    .map_err(|e| format!("Failed to enable Service Mode: {}", e))?;
            } else {
                println!("Service Mode LaunchDaemon is already loaded, skipping enable step");
            }
        }
    }
    
    // Update desired mode (memory)
    {
        let mut desired = state.desired_mode.lock().map_err(|e| e.to_string())?;
        *desired = target_mode;
    }
    
    // Persist mode preference for next app launch
    if let Err(e) = crate::user_overrides::persist_core_mode(&mode) {
        eprintln!("Warning: Failed to persist core mode preference: {}", e);
    }
    
    // If core is running, restart with new mode
    if is_core_running(state.inner()) {
        println!("Core is running, restarting with new mode: {:?}", target_mode);
        
        let config_path = {
            state.config_path.lock()
                .ok()
                .and_then(|lock| lock.clone())
                .or_else(|| {
                    crate::profiles::get_active_profile_path()
                        .ok()
                        .flatten()
                        .map(PathBuf::from)
                })
                .unwrap_or_else(|| get_config_dir().join("config.yaml"))
        };
        
        // Stop current mode
        stop_core_inner(state.inner()).await?;
        
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // Start with new mode
        let options = StartOptions {
            config_path: Some(config_path.to_string_lossy().to_string()),
            external_controller: None,
            use_root: None,
            mode: Some(target_mode),
        };
        
        start_core_inner(state.clone(), Some(options)).await?;
    } else {
        // Core is not running, auto-start with the new mode
        println!("Core is not running; auto-starting in {:?} mode...", target_mode);

        let config_path = crate::profiles::get_active_profile_path()
            .ok()
            .flatten()
            .map(PathBuf::from)
            .unwrap_or_else(|| get_config_dir().join("config.yaml"));

        let options = StartOptions {
            config_path: Some(config_path.to_string_lossy().to_string()),
            external_controller: None,
            use_root: None,
            mode: Some(target_mode),
        };

        start_core_inner(state.clone(), Some(options)).await?;

        // Notify frontend so Dashboard "Start" flips to "Stop" immediately.
        let _ = app.emit(
            "core-started",
            CoreStartedEvent {
                success: true,
                message: None,
            },
        );
    }
    
    // Emit event after successful mode change
    let _ = app.emit("core-mode-changed", CoreModeChangedEvent { mode: mode.clone() });
    
    Ok(())
}
