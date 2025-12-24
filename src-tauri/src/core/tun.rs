// ========== TUN Mode ==========

// ========== macOS System DNS Management ==========
// When TUN mode is enabled, set system DNS to public DNS to ensure DNS resolution works.
// This is a key feature from clash-verge-rev that fixes DNS issues in TUN mode.

/// Find a script in multiple possible locations (dev mode and release mode)
#[cfg(target_os = "macos")]
fn find_script(app: &tauri::AppHandle, script_name: &str) -> Option<std::path::PathBuf> {
    use tauri::Manager;
    
    // Try resource_dir first (release mode)
    if let Ok(dir) = app.path().resource_dir() {
        let path = dir.join(script_name);
        if path.exists() {
            return Some(path);
        }
    }
    
    // Try relative to executable (release mode alternative)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let path = exe_dir.join(script_name);
            if path.exists() {
                return Some(path);
            }
            // Also try ../Resources for macOS app bundle
            let path = exe_dir.join("../Resources").join(script_name);
            if path.exists() {
                return Some(path);
            }
        }
    }
    
    // Try dev mode paths: relative to project root
    let dev_paths = [
        format!("resources/{}", script_name),
        format!("../resources/{}", script_name),
        format!("../../resources/{}", script_name),
    ];
    
    for dev_path in &dev_paths {
        let path = std::path::PathBuf::from(dev_path);
        if path.exists() {
            return Some(path);
        }
    }
    
    // Try from CARGO_MANIFEST_DIR if in dev
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let path = std::path::PathBuf::from(&manifest_dir)
            .parent()
            .map(|p| p.join("resources").join(script_name));
        if let Some(p) = path {
            if p.exists() {
                return Some(p);
            }
        }
    }
    
    None
}

#[cfg(target_os = "macos")]
async fn set_system_dns(app: &tauri::AppHandle, dns_server: &str) {
    use tauri_plugin_shell::ShellExt;
    
    let script_path = match find_script(app, "set_dns.sh") {
        Some(p) => p,
        None => {
            println!("DNS setup: set_dns.sh not found in any search path");
            return;
        }
    };

    // IMPORTANT: always execute with an absolute path.
    // In dev, `find_script` may return a relative path like `resources/set_dns.sh`.
    // If we set current_dir to `resources` and still pass `resources/set_dns.sh`,
    // it becomes `resources/resources/set_dns.sh` and fails.
    let script_abs = script_path
        .canonicalize()
        .unwrap_or_else(|_| script_path.clone());

    println!(
        "DNS setup: Setting system DNS to {} using {:?}",
        dns_server, script_abs
    );

    // Following clash-verge-rev's approach: use tauri_plugin_shell, no sudo needed.
    let output = app
        .shell()
        .command("bash")
        .args([script_abs.to_string_lossy().to_string(), dns_server.to_string()])
        .output()
        .await;

    match output {
        Ok(output) => {
            if output.status.success() {
                println!("DNS setup: Successfully set system DNS to {}", dns_server);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                println!("DNS setup: Failed to set system DNS: stderr={}, stdout={}", stderr, stdout);
            }
        }
        Err(e) => {
            println!("DNS setup: Failed to execute set_dns.sh: {}", e);
        }
    }
}

#[cfg(target_os = "macos")]
async fn restore_system_dns(app: &tauri::AppHandle) {
    use tauri_plugin_shell::ShellExt;
    
    let script_path = match find_script(app, "unset_dns.sh") {
        Some(p) => p,
        None => {
            println!("DNS restore: unset_dns.sh not found in any search path");
            return;
        }
    };

    let script_abs = script_path
        .canonicalize()
        .unwrap_or_else(|_| script_path.clone());

    println!(
        "DNS restore: Restoring original system DNS using {:?}",
        script_abs
    );

    let output = app
        .shell()
        .command("bash")
        .args([script_abs.to_string_lossy().to_string()])
        .output()
        .await;

    match output {
        Ok(output) => {
            if output.status.success() {
                println!("DNS restore: Successfully restored system DNS");
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                println!("DNS restore: Failed to restore system DNS: stderr={}, stdout={}", stderr, stdout);
            }
        }
        Err(e) => {
            println!("DNS restore: Failed to execute unset_dns.sh: {}", e);
        }
    }
}

/// Restart LaunchDaemon using osascript (requires password)
/// This is a fallback when PATCH API is not available (older Mihomo versions)
#[cfg(target_os = "macos")]
#[allow(dead_code)]
async fn restart_launchdaemon_with_osascript() -> Result<(), String> {
    const SERVICE_LABEL: &str = "com.aqiu.service";
    const SERVICE_PLIST_PATH: &str = "/Library/LaunchDaemons/com.aqiu.service.plist";
    
    println!("LaunchDaemon restart: Using osascript with admin privileges...");
    
    // Use osascript to run launchctl commands with admin privileges
    let restart_script = format!(
        r#"do shell script "launchctl bootout system/{} 2>/dev/null; sleep 1; launchctl bootstrap system '{}'" with administrator privileges"#,
        SERVICE_LABEL, SERVICE_PLIST_PATH
    );
    
    let restart_result = std::process::Command::new("osascript")
        .args(["-e", &restart_script])
        .output();
    
    match restart_result {
        Ok(output) if output.status.success() => {
            println!("LaunchDaemon restart: Restarted successfully with admin privileges");
            Ok(())
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Check if user cancelled the authorization
            if stderr.contains("-128") || stderr.contains("User canceled") {
                println!("LaunchDaemon restart: User cancelled authorization");
                return Err("User cancelled authorization".into());
            }
            println!("LaunchDaemon restart: Returned: stderr={}, stdout={}", stderr, stdout);
            // Try kickstart as fallback
            let kickstart_script = format!(
                r#"do shell script "launchctl kickstart -k system/{}" with administrator privileges"#,
                SERVICE_LABEL
            );
            match std::process::Command::new("osascript")
                .args(["-e", &kickstart_script])
                .output()
            {
                Ok(output) if output.status.success() => {
                    println!("LaunchDaemon restart: Kickstart successful");
                    Ok(())
                }
                _ => Err("Failed to restart LaunchDaemon".into())
            }
        }
        Err(e) => {
            println!("LaunchDaemon restart: Failed: {}", e);
            Err(format!("Failed to restart LaunchDaemon: {}", e))
        }
    }
}

/// Enable or disable TUN mode via Mihomo API
#[tauri::command]
pub async fn set_tun_mode(
    app: tauri::AppHandle,
    state: tauri::State<'_, MihomoState>,
    enable: bool,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    if enable && !is_privileged_helper_valid() {
        println!("TUN Mode: Helper not installed, attempting auto-installation...");
        install_privileged_helper(app.clone(), state.clone())
            .await
            .map_err(|e| {
                format!(
                    "Failed to install privileged helper required for TUN mode: {}",
                    e
                )
            })?;
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        // 1. Persist the user's preference FIRST
        if let Err(err) = crate::user_overrides::persist_tun_override(enable) {
            return Err(format!("Failed to save TUN preference: {}", err));
        }

        // 2. Check if running and what mode
        let was_running = {
            #[cfg(target_os = "macos")]
            {
                is_core_running(state.inner())
            }
            #[cfg(target_os = "linux")]
            {
                let mut process_lock = state.process.lock().map_err(|e| e.to_string())?;
                if let Some(child) = process_lock.as_mut() {
                    matches!(child.try_wait(), Ok(None))
                } else {
                    false
                }
            }
        };

        // 3. If running, apply the new TUN setting
        if was_running {
            println!("TUN mode change: Core is running, applying new TUN setting...");
            
            #[cfg(target_os = "macos")]
            {
                // Check current mode
                let current_mode = state.current_mode.lock().ok().map(|m| *m);
                
                if matches!(current_mode, Some(CoreMode::Service)) {
                    // Service Mode: TUN changes require restart via official API
                    println!("TUN mode change: Service Mode detected, restarting via mihomo API...");
                    
                    // Build new config with TUN override and write to system path
                    let config_path = resolve_config_path(state.inner());
                    let content = std::fs::read_to_string(&config_path)
                        .map_err(|e| format!("Failed to read config: {}", e))?;
                    
                    let final_content = match serde_yaml::from_str::<serde_yaml::Value>(&content) {
                        Ok(mut yaml) => {
                            // DEBUG: Log overrides file path and content
                            let overrides_path = dirs::data_local_dir()
                                .unwrap_or_default()
                                .join("aqiu")
                                .join("user_overrides.json");
                            println!("TUN mode change: Loading overrides from {:?}", overrides_path);
                            if let Ok(overrides_content) = std::fs::read_to_string(&overrides_path) {
                                println!("TUN mode change: Overrides content: {}", overrides_content);
                            } else {
                                println!("TUN mode change: WARNING - Could not read overrides file!");
                            }
                            
                            let overrides = crate::user_overrides::load_overrides();
                            println!("TUN mode change: Loaded overrides - TUN enable: {:?}", 
                                overrides.tun.as_ref().and_then(|t| t.enable));
                            if let Err(e) = crate::user_overrides::apply_overrides_to_yaml(&mut yaml, &overrides) {
                                eprintln!("Warning: Failed to apply user overrides: {}", e);
                                content.clone()
                            } else {
                                // DEBUG: Verify TUN section after applying overrides
                                if let Some(tun) = yaml.get("tun") {
                                    println!("TUN mode change: After apply - tun.enable = {:?}", 
                                        tun.get("enable").and_then(|v| v.as_bool()));
                                }
                                serde_yaml::to_string(&yaml).unwrap_or(content.clone())
                            }
                        }
                        Err(_) => content.clone(),
                    };
                    
                    // Write to system config path
                    const SYSTEM_CONFIG_PATH: &str = "/Library/Application Support/aqiu/config.yaml";
                    
                    // Debug: Log the TUN and DNS sections being written
                    if let Ok(yaml_check) = serde_yaml::from_str::<serde_yaml::Value>(&final_content) {
                        if let Some(tun_section) = yaml_check.get("tun") {
                            println!("TUN mode change: Writing TUN config:");
                            if let Some(enable_val) = tun_section.get("enable") {
                                println!("  - enable: {:?}", enable_val);
                            }
                            if let Some(stack_val) = tun_section.get("stack") {
                                println!("  - stack: {:?}", stack_val);
                            }
                            if let Some(hijack_val) = tun_section.get("dns-hijack") {
                                println!("  - dns-hijack: {:?}", hijack_val);
                            }
                            
                            if let Some(dns_section) = yaml_check.get("dns") {
                                println!("TUN mode change: DNS config:");
                                if let Some(enable_val) = dns_section.get("enable") {
                                    println!("  - enable: {:?}", enable_val);
                                }
                                if let Some(mode_val) = dns_section.get("enhanced-mode") {
                                    println!("  - enhanced-mode: {:?}", mode_val);
                                }
                                if let Some(listen_val) = dns_section.get("listen") {
                                    println!("  - listen: {:?}", listen_val);
                                }
                                if let Some(nameserver_val) = dns_section.get("nameserver") {
                                    println!("  - nameserver count: {}", 
                                        nameserver_val.as_sequence().map(|s| s.len()).unwrap_or(0));
                                }
                            } else {
                                println!("TUN mode change: WARNING - No DNS section in config!");
                            }
                        } else {
                            println!("TUN mode change: WARNING - No TUN section in final config!");
                        }
                    }
                    
                    // Write config to system path
                    // Note: This path should be writable by the current user (set during service install)
                    if let Err(e) = std::fs::write(SYSTEM_CONFIG_PATH, &final_content) {
                        println!("TUN mode change: Failed to write to system config: {}", e);
                        println!("TUN mode change: Checking file permissions...");
                        
                        // Try to get file info for debugging
                        if let Ok(metadata) = std::fs::metadata(SYSTEM_CONFIG_PATH) {
                            println!("TUN mode change: Config file exists, readonly={}", metadata.permissions().readonly());
                        } else {
                            println!("TUN mode change: Config file does not exist or cannot be accessed");
                        }
                        
                        // Check if directory is writable
                        let test_path = "/Library/Application Support/aqiu/.write_test";
                        if std::fs::write(test_path, "test").is_ok() {
                            let _ = std::fs::remove_file(test_path);
                            println!("TUN mode change: Directory is writable but config file is not");
                        } else {
                            println!("TUN mode change: Directory is not writable - permission issue");
                        }
                        
                        return Err(format!("Failed to write system config: {}. Try reinstalling Service Mode.", e));
                    }
                    println!("TUN mode change: Successfully wrote config to {}", SYSTEM_CONFIG_PATH);

                    // Keep runtime config in sync for debugging / User Mode switching.
                    // This also addresses reports that `config.runtime.yaml` shows `tun.enable: false`
                    // after toggling TUN in Service Mode.
                    {
                        let runtime_dir = get_config_dir();
                        let _ = std::fs::create_dir_all(&runtime_dir);
                        let runtime_path = runtime_dir.join("config.runtime.yaml");
                        if let Err(e) = std::fs::write(&runtime_path, &final_content) {
                            println!(
                                "TUN mode change: Failed to sync runtime config {:?}: {}",
                                runtime_path, e
                            );
                        } else {
                            println!(
                                "TUN mode change: Synced runtime config: {:?}",
                                runtime_path
                            );
                        }
                    }
                    
                    // Get API credentials
                    let (api_host, api_port, api_secret) = {
                        let host = state.api_host.lock().map_err(|e| e.to_string())?.clone();
                        let port = *state.api_port.lock().map_err(|e| e.to_string())?;
                        let secret = get_api_secret_from_state(state.inner());
                        (host, port, secret)
                    };
                    
                    // Use PUT /configs?force=true to reload entire config from file
                    // 
                    // NOTE: We cannot use PATCH API because:
                    // - PATCH only modifies TUN settings in memory
                    // - DNS module is initialized at startup and cannot be dynamically enabled
                    // - dns-hijack requires DNS to be listening, which needs full config reload
                    //
                    // PUT /configs?force=true will:
                    // 1. Re-read the config file (which we already updated with TUN settings)
                    // 2. Reinitialize DNS module
                    // 3. Reinitialize TUN interface
                    // 4. Does NOT require password (service already runs as root)
                    let client = reqwest::Client::new();
                    let reload_url = format!("http://{}:{}/configs?force=true", api_host, api_port);
                    let mut req = client.put(&reload_url);
                    if let Some(s) = &api_secret {
                        req = req.header("Authorization", format!("Bearer {}", s));
                    }
                    
                    // Specify the config path to reload
                    let payload = serde_json::json!({
                        "path": SYSTEM_CONFIG_PATH
                    });
                    req = req.json(&payload);
                    
                    println!("TUN mode change: Reloading config via PUT /configs?force=true");
                    println!("TUN mode change: Config path: {}", SYSTEM_CONFIG_PATH);
                    
                    let resp = req.send().await;
                    
                    match resp {
                        Ok(r) if r.status().is_success() => {
                            println!("TUN mode change: Config reload API returned success");
                        }
                        Ok(r) => {
                            let status = r.status();
                            let error_text = r.text().await.unwrap_or_default();
                            println!("TUN mode change: Config reload returned status {} - {}", status, error_text);
                        }
                        Err(e) => {
                            println!("TUN mode change: Config reload API request failed: {}", e);
                            return Err(format!("Failed to reload config: {}", e));
                        }
                    }
                    
                    // Wait for TUN interface and DNS to initialize
                    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                    println!("TUN mode change: Initialization wait completed");
                    
                    // Set or restore system DNS based on TUN mode
                    // Following clash-verge-rev's approach:
                    // - TUN's dns-hijack intercepts all DNS queries to port 53
                    // - Setting system DNS to a public DNS (223.6.6.6) ensures DNS queries
                    //   go through the network and get hijacked by TUN
                    // - This is more reliable than 127.0.0.1 which requires dns.listen to work
                    if enable {
                        // Set system DNS to public DNS (will be hijacked by TUN)
                        set_system_dns(&app, "223.6.6.6").await;
                    } else {
                        // Restore original system DNS
                        restore_system_dns(&app).await;
                    }
                    
                    // Notify frontend
                    let _ = app.emit(
                        "core-started",
                        serde_json::json!({ "success": true, "message": Some(format!("Core restarted with TUN mode {}", if enable { "enabled" } else { "disabled" })) }),
                    );
                    println!("TUN mode change: Completed successfully");
                } else {
                    // User Mode: Need to restart (TUN in User Mode will be rejected anyway)
                    println!("TUN mode change: User Mode detected, restarting core...");
                    stop_core_inner(state.inner()).await?;
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    
                    let config_path = resolve_config_path(state.inner());
                    let options = StartOptions {
                        config_path: Some(config_path.to_string_lossy().to_string()),
                        external_controller: None,
                        use_root: None,
                        mode: None,
                    };
                    
                    if let Err(err) = start_core_inner(state.clone(), Some(options)).await {
                        let _ = crate::user_overrides::persist_tun_override(!enable);
                        return Err(format!(
                            "Failed to restart Mihomo after {} TUN mode: {}",
                            describe_tun_action(enable),
                            err
                        ));
                    }
                    
                    // Set or restore system DNS for User Mode too
                    // Following clash-verge-rev's approach: use public DNS for TUN
                    if enable {
                        set_system_dns(&app, "223.6.6.6").await;
                    } else {
                        restore_system_dns(&app).await;
                    }
                    
                    let _ = app.emit(
                        "core-started",
                        serde_json::json!({ "success": true, "message": Some(format!("Core restarted with TUN mode {}", if enable { "enabled" } else { "disabled" })) }),
                    );
                    println!("TUN mode change: Core restarted successfully");
                }
            }
            
            #[cfg(target_os = "linux")]
            {
                // Linux: Restart with sudo if TUN enabled
                stop_core_inner(state.inner()).await?;
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                
                let config_path = resolve_config_path(state.inner());
                let options = StartOptions {
                    config_path: Some(config_path.to_string_lossy().to_string()),
                    external_controller: None,
                    use_root: Some(enable),
                };
                
                if let Err(err) = start_core_inner(state.clone(), Some(options)).await {
                    let _ = crate::user_overrides::persist_tun_override(!enable);
                    return Err(format!(
                        "Failed to restart Mihomo after {} TUN mode: {}",
                        describe_tun_action(enable),
                        err
                    ));
                }
                
                let _ = app.emit(
                    "core-started",
                    serde_json::json!({ "success": true, "message": Some(format!("Core restarted with TUN mode {}", if enable { "enabled" } else { "disabled" })) }),
                );
                println!("TUN mode change: Core restarted successfully");
            }
        } else {
            println!("TUN mode change: Core is not running, saved preference for next start");
        }
        
        // Emit event after successful TUN mode change
        let _ = app.emit("tun-mode-changed", TunModeChangedEvent { enabled: enable });
        
        return Ok(());
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let previous = get_tun_status(state.clone()).await.unwrap_or(!enable);
        let (api_host, api_port, api_secret) = {
            let host = state.api_host.lock().map_err(|e| e.to_string())?.clone();
            let port = *state.api_port.lock().map_err(|e| e.to_string())?;
            let secret = get_api_secret_from_state(state.inner());
            (host, port, secret)
        };

        let url = format!("http://{}:{}/configs", api_host, api_port);

        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "tun": {
                "enable": enable
            }
        });

        let request = add_auth_header(
            client.patch(&url).json(&payload).timeout(std::time::Duration::from_secs(5)),
            api_secret.as_deref()
        );
        let response = request.send().await
            .map_err(|e| format!("Failed to {} TUN mode: {}", describe_tun_action(enable), e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Failed to {} TUN mode: {}",
                describe_tun_action(enable),
                response.status()
            ));
        }

        if let Err(err) = crate::user_overrides::persist_tun_override(enable) {
            let revert_payload = serde_json::json!({
                "tun": {
                    "enable": previous
                }
            });
            let revert_request = add_auth_header(
                client.patch(&url).json(&revert_payload).timeout(std::time::Duration::from_secs(5)),
                api_secret.as_deref()
            );
            let _ = revert_request.send().await;
            return Err(format!("Failed to save TUN preference: {}", err));
        }

        Ok(())
    }
}

/// Get current TUN mode status from Mihomo API
#[tauri::command]
pub async fn get_tun_status(state: tauri::State<'_, MihomoState>) -> Result<bool, String> {
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
    let response = request.send().await;

    let response = match response {
        Ok(res) => res,
        Err(_e) => {
            #[cfg(any(target_os = "macos", target_os = "linux"))]
            {
                let config_path = resolve_config_path(state.inner());
                return Ok(read_tun_from_config(&config_path).unwrap_or(false));
            }
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            {
                return Err(format!("Failed to get TUN status: {}", _e));
            }
        }
    };

    if !response.status().is_success() {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let config_path = resolve_config_path(state.inner());
            return Ok(read_tun_from_config(&config_path).unwrap_or(false));
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            return Ok(false);
        }
    }
    let config: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    Ok(config
        .get("tun")
        .and_then(|tun| tun.get("enable"))
        .and_then(|enable| enable.as_bool())
        .unwrap_or(false))
}

