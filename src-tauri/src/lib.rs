// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

mod core;
mod profiles;
mod service;
mod user_overrides;

use core::MihomoState;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, State,
};
use tauri_plugin_autostart::MacosLauncher;

struct TrayMenuState {
    system_proxy: CheckMenuItem<tauri::Wry>,
    tun_mode: CheckMenuItem<tauri::Wry>,
    mode_global: CheckMenuItem<tauri::Wry>,
    mode_rule: CheckMenuItem<tauri::Wry>,
    mode_direct: CheckMenuItem<tauri::Wry>,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn update_tray_state(
    state: State<'_, TrayMenuState>,
    system_proxy: bool,
    tun_mode: bool,
    mode: String,
) -> Result<(), String> {
    state
        .system_proxy
        .set_checked(system_proxy)
        .map_err(|e| e.to_string())?;
    state
        .tun_mode
        .set_checked(tun_mode)
        .map_err(|e| e.to_string())?;

    let normalized = mode.to_lowercase();
    let (is_global, is_rule, is_direct) = match normalized.as_str() {
        "global" => (true, false, false),
        "direct" => (false, false, true),
        _ => (false, true, false),
    };

    state
        .mode_global
        .set_checked(is_global)
        .map_err(|e| e.to_string())?;
    state
        .mode_rule
        .set_checked(is_rule)
        .map_err(|e| e.to_string())?;
    state
        .mode_direct
        .set_checked(is_direct)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn update_tray_title(app: tauri::AppHandle, title: String) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_title(Some(title)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn update_tray_icon(app: tauri::AppHandle, bytes: Vec<u8>) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id("main") {
        let image = tauri::image::Image::from_bytes(&bytes).map_err(|e| e.to_string())?;
        tray.set_icon(Some(image)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn restore_tray_icon(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id("main") {
        let icon = app.default_window_icon().cloned();
        tray.set_icon(icon).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ========== Service IPC Commands ==========

#[derive(serde::Serialize)]
struct ServiceStatusResult {
    status: String,
    version: Option<String>,
    needs_reinstall: bool,
}

#[tauri::command]
async fn service_check_status() -> Result<ServiceStatusResult, String> {
    let status = service::check_service_status().await;

    let (status_str, needs_reinstall) = match &status {
        service::ServiceStatus::Ready => ("ready".to_string(), false),
        service::ServiceStatus::NeedsReinstall => ("needs_reinstall".to_string(), true),
        service::ServiceStatus::NotInstalled => ("not_installed".to_string(), false),
        service::ServiceStatus::Unavailable(e) => (format!("unavailable: {}", e), false),
    };

    let version = if status == service::ServiceStatus::Ready {
        service::get_version().await.ok()
    } else {
        None
    };

    Ok(ServiceStatusResult {
        status: status_str,
        version,
        needs_reinstall,
    })
}

#[tauri::command]
async fn service_get_version() -> Result<String, String> {
    service::get_version().await
}

#[tauri::command]
async fn service_install(app: tauri::AppHandle) -> Result<(), String> {
    service::install_service(&app).await
}

#[tauri::command]
async fn service_uninstall(app: tauri::AppHandle) -> Result<(), String> {
    service::uninstall_service(&app).await
}

#[tauri::command]
async fn service_ping() -> Result<bool, String> {
    Ok(service::is_service_available().await)
}

fn create_tray(app: &tauri::App) -> Result<TrayMenuState, Box<dyn std::error::Error>> {
    let show_item = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    // System Proxy & Tun
    let system_proxy = CheckMenuItem::with_id(
        app,
        "system_proxy",
        "System Proxy",
        true,
        false,
        None::<&str>,
    )?;
    let tun_mode = CheckMenuItem::with_id(app, "tun_mode", "Tun Mode", true, false, None::<&str>)?;

    // Mode Submenu
    let mode_global =
        CheckMenuItem::with_id(app, "mode_global", "Global", true, false, None::<&str>)?;
    let mode_rule = CheckMenuItem::with_id(app, "mode_rule", "Rule", true, true, None::<&str>)?;
    let mode_direct =
        CheckMenuItem::with_id(app, "mode_direct", "Direct", true, false, None::<&str>)?;

    let mode_submenu =
        Submenu::with_items(app, "Mode", true, &[&mode_global, &mode_rule, &mode_direct])?;

    // Nodes Submenu (Placeholder for now)
    let open_dashboard =
        MenuItem::with_id(app, "open_dashboard", "Open Dashboard", true, None::<&str>)?;
    let copy_proxy_cmd = MenuItem::with_id(
        app,
        "copy_proxy_cmd",
        "Copy Proxy Command",
        true,
        None::<&str>,
    )?;
    let nodes_submenu =
        Submenu::with_items(app, "Nodes", true, &[&open_dashboard, &copy_proxy_cmd])?;

    let menu = Menu::with_items(
        app,
        &[
            &show_item,
            &separator,
            &system_proxy,
            &tun_mode,
            &separator,
            &mode_submenu,
            &nodes_submenu,
            &separator,
            &quit_item,
        ],
    )?;

    let _tray = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            match event.id.as_ref() {
                "show" | "open_dashboard" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "system_proxy" => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        // Toggle logic would require knowing current state or just toggling based on menu item state if we trust it
                        // But better to check real state or just toggle.
                        // For now, let's try to get the menu item and toggle it.
                        // Since we can't easily get the menu item instance here without storing it,
                        // we might need to rely on the frontend or just check the current status.

                        // Let's check status first
                        let status = core::get_system_proxy_status().unwrap_or(false);
                        let new_status = !status;
                        let _ = core::set_system_proxy(app_handle.clone(), new_status, None).await;

                        // Update menu item check state?
                        // We need to find the menu item by ID to update it.
                        // This is a bit complex in the closure.
                        // We can emit an event to frontend to refresh everything,
                        // or try to update the menu if we had a handle to it.
                    });
                }
                "tun_mode" => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let state = app_handle.state::<MihomoState>();
                        let status = core::get_tun_status(state.clone()).await.unwrap_or(false);
                        let _ = core::set_tun_mode(app_handle.clone(), state, !status).await;
                    });
                }
                "mode_global" => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let state = app_handle.state::<MihomoState>();
                        let _ =
                            core::set_mode(app_handle.clone(), state, "global".to_string()).await;
                    });
                }
                "mode_rule" => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let state = app_handle.state::<MihomoState>();
                        let _ = core::set_mode(app_handle.clone(), state, "rule".to_string()).await;
                    });
                }
                "mode_direct" => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let state = app_handle.state::<MihomoState>();
                        let _ =
                            core::set_mode(app_handle.clone(), state, "direct".to_string()).await;
                    });
                }
                "copy_proxy_cmd" => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let state = app_handle.state::<MihomoState>();
                        let _ = core::copy_proxy_env(state).await;
                    });
                }
                "quit" => {
                    let app_handle = app.app_handle().clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = core::set_system_proxy(app_handle.clone(), false, None).await;
                        let state = app_handle.state::<MihomoState>();
                        let _ = core::stop_core_inner(state.inner()).await;
                        app_handle.exit(0);
                    });
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(TrayMenuState {
        system_proxy,
        tun_mode,
        mode_global,
        mode_rule,
        mode_direct,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .manage(MihomoState::default())
        .setup(|app| {
            let tray_state = create_tray(app)?;
            app.manage(tray_state);

            // On startup: restore core mode preference, recover orphaned core, then auto-start if needed
            #[cfg(target_os = "macos")]
            {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<MihomoState>();
                    
                    // Step 0: Restore persisted core mode preference
                    if let Some(persisted_mode) = user_overrides::get_persisted_core_mode() {
                        let target_mode = match persisted_mode.as_str() {
                            "service" => core::CoreMode::Service,
                            _ => core::CoreMode::User,
                        };
                        if let Ok(mut desired) = state.desired_mode.lock() {
                            *desired = target_mode;
                            println!("Startup: Restored core mode preference: {:?}", target_mode);
                        }
                        if let Ok(mut current) = state.current_mode.lock() {
                            *current = target_mode;
                        }
                    }
                    
                    // Step 1: Check for orphaned core process from previous crash
                    let recovered = core::recover_orphaned_core(state.clone())
                        .await
                        .unwrap_or(false);
                    
                    if recovered {
                        println!("Startup: Recovered orphaned core process, skipping auto-start");
                        return;
                    }
                    
                    // Step 2: Auto-start core on app launch
                    // For Service Mode: requires privileged helper to be installed
                    // For User Mode: start directly
                    let persisted_mode = user_overrides::get_persisted_core_mode();
                    let is_service_mode = persisted_mode.as_deref() == Some("service");
                    
                    let should_auto_start = if is_service_mode {
                        // Service Mode requires helper to be installed
                        core::get_privileged_helper_status().await.unwrap_or(false)
                    } else {
                        // User Mode: always auto-start
                        true
                    };
                    
                    if should_auto_start {
                        println!("Startup: Auto-starting core in {:?} mode...", 
                            if is_service_mode { "Service" } else { "User" });
                        let start_result = core::start_core(app_handle.clone(), state.clone(), None).await;
                        
                        // After core starts successfully, check GEO database
                        if start_result.is_ok() {
                            // Wait a bit for core to fully initialize
                            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                            
                            // GEO files auto-download is DISABLED
                            // User should manually sync GEO files via Settings if needed
                            let config_dir = core::get_config_dir();
                            let geoip_path = config_dir.join("geoip.dat");
                            let geosite_path = config_dir.join("geosite.dat");
                            let geoip_exists = geoip_path.exists();
                            let geosite_exists = geosite_path.exists();
                            
                            if !geoip_exists || !geosite_exists {
                                println!("Startup: GEO database incomplete:");
                                if !geoip_exists {
                                    println!("  - geoip.dat not found at: {:?}", geoip_path);
                                }
                                if !geosite_exists {
                                    println!("  - geosite.dat not found at: {:?}", geosite_path);
                                }
                                println!("Startup: Auto-download disabled. Use Settings -> Update GEO to download manually.");
                            } else {
                                println!("Startup: GEO database exists:");
                                println!("  - geoip.dat: {:?}", geoip_path);
                                println!("  - geosite.dat: {:?}", geosite_path);
                            }
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            update_tray_state,
            update_tray_title,
            update_tray_icon,
            restore_tray_icon,
            core::start_core,
            core::stop_core,
            core::restart_core,
            core::get_core_status,
            core::set_system_proxy,
            core::get_system_proxy_status,
            core::set_tun_mode,
            core::get_tun_status,
            core::set_mode,
            core::get_mode,
            core::copy_proxy_env,
            core::download_core,
            core::download_geodata,
            core::import_core_binary,
            core::check_core_exists,
            core::get_app_paths,
            core::download_profile,
            #[cfg(target_os = "macos")]
            core::get_privileged_helper_status,
            #[cfg(target_os = "macos")]
            core::install_privileged_helper,
            #[cfg(target_os = "macos")]
            core::uninstall_privileged_helper,
            #[cfg(target_os = "macos")]
            core::get_core_mode,
            #[cfg(target_os = "macos")]
            core::get_desired_core_mode,
            #[cfg(target_os = "macos")]
            core::set_core_mode,
            #[cfg(target_os = "macos")]
            core::recover_orphaned_core,

            profiles::list_profiles,
            profiles::get_active_profile,
            profiles::create_profile,
            profiles::create_profile_from_path,
            profiles::delete_profile,
            profiles::set_active_profile,
            profiles::get_profile_content,
            profiles::save_profile_content,
            profiles::rename_profile,
            profiles::update_profile_from_url,
            profiles::parse_config,
            profiles::save_config_obj,
            profiles::add_proxy_to_profile,
            profiles::parse_proxy_url,
            profiles::get_active_profile_path,
            user_overrides::set_user_override,
            user_overrides::get_user_overrides,
            user_overrides::clear_user_overrides,
            // Service IPC commands
            service_check_status,
            service_get_version,
            service_install,
            service_uninstall,
            service_ping,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                let app_handle_clone = app_handle.clone();
                // Use block_on to ensure cleanup finishes before process exits
                tauri::async_runtime::block_on(async move {
                    // Turn off system proxy on exit
                    let _ = core::set_system_proxy(app_handle_clone.clone(), false, None).await;
                    
                    // Get state reference for core operations
                    let state = app_handle.state::<MihomoState>();
                    
                    #[cfg(target_os = "macos")]
                    {
                        // On app exit, completely stop service mode via API (no password prompt)
                        let _ = core::force_stop_service_mode_on_exit(state.inner()).await;
                    }
                    
                    // Stop the core (user mode or any remaining process)
                    let _ = core::stop_core_inner(state.inner()).await;
                });
            }
        });
}
