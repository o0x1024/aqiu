import { invoke } from '@tauri-apps/api/core';

// ========== Types ==========

export interface CoreStatus {
    running: boolean;
    version: string | null;
    config_path: string | null;
    api_host: string;
    api_port: number;
    api_endpoint: string;
    api_secret: string | null;
    uptime_seconds: number | null;
    message: string | null;
}

export interface StartOptions {
    config_path?: string;
    external_controller?: string;
    mode?: 'user' | 'service';
}

export type CoreMode = 'user' | 'service';

export interface AppPaths {
    mihomo_binary: string;
    config_dir: string;
    data_dir: string;
}

// ========== Core Management ==========

/**
 * Start the Mihomo core process
 */
export async function startCore(options?: StartOptions): Promise<CoreStatus> {
    return await invoke<CoreStatus>('start_core', { options });
}

/**
 * Stop the Mihomo core process
 */
export async function stopCore(): Promise<void> {
    return await invoke('stop_core');
}

/**
 * Restart the Mihomo core process
 */
export async function restartCore(): Promise<CoreStatus> {
    return await invoke<CoreStatus>('restart_core');
}

/**
 * Get the current core status
 */
let coreStatusInFlight: Promise<CoreStatus> | null = null;
const CORE_STATUS_TIMEOUT_MS = 8000;

export async function getCoreStatus(): Promise<CoreStatus> {
    if (coreStatusInFlight) return coreStatusInFlight;

    coreStatusInFlight = (async () => {
        try {
            const invokePromise = invoke<CoreStatus>('get_core_status');
            const timeoutPromise = new Promise<CoreStatus>((_, reject) => {
                setTimeout(() => reject(new Error('get_core_status timeout')), CORE_STATUS_TIMEOUT_MS);
            });
            return await Promise.race([invokePromise, timeoutPromise]);
        } finally {
            coreStatusInFlight = null;
        }
    })();

    return coreStatusInFlight;
}

// ========== System Proxy ==========

/**
 * Enable or disable system proxy
 */
export async function setSystemProxy(enable: boolean, port?: number): Promise<void> {
    return await invoke('set_system_proxy', { enable, port });
}

/**
 * Get current system proxy status
 */
export async function getSystemProxyStatus(): Promise<boolean> {
    return await invoke<boolean>('get_system_proxy_status');
}

// ========== TUN Mode ==========

/**
 * Enable or disable TUN mode
 */
export async function setTunMode(enable: boolean): Promise<void> {
    return await invoke('set_tun_mode', { enable });
}

/**
 * Get current TUN mode status
 */
export async function getTunStatus(): Promise<boolean> {
    return await invoke<boolean>('get_tun_status');
}

// ========== Proxy Mode ==========

/**
 * Set proxy mode
 */
export async function setMode(mode: string): Promise<void> {
    return await invoke('set_mode', { mode });
}

/**
 * Get current proxy mode
 */
export async function getMode(): Promise<string> {
    return await invoke<string>('get_mode');
}

// ========== Tray Management ==========

/**
 * Update tray menu state
 */
export async function updateTrayState(systemProxy: boolean, tunMode: boolean, mode: string): Promise<void> {
    return await invoke('update_tray_state', { systemProxy, tunMode, mode });
}

/**
 * Update tray title (macOS status bar)
 */
export async function updateTrayTitle(title: string): Promise<void> {
    return await invoke('update_tray_title', { title });
}

/**
 * Update tray icon from bytes
 */
export async function updateTrayIcon(bytes: Uint8Array | number[]): Promise<void> {
    return await invoke('update_tray_icon', { bytes: Array.from(bytes) });
}

/**
 * Restore tray icon to default
 */
export async function restoreTrayIcon(): Promise<void> {
    return await invoke('restore_tray_icon');
}

// ========== Core Binary Management ==========

/**
 * Check if Mihomo binary exists
 */
export async function checkCoreExists(): Promise<boolean> {
    return await invoke<boolean>('check_core_exists');
}

/**
 * Download Mihomo core binary
 */
export async function downloadCore(version?: string): Promise<string> {
    return await invoke<string>('download_core', { version });
}

/**
 * Download GeoIP and GeoSite database files
 */
export async function downloadGeodata(): Promise<string> {
    return await invoke<string>('download_geodata');
}

/**
 * Import Mihomo core binary from local path
 */
export async function importCoreBinary(path: string): Promise<string> {
    return await invoke<string>('import_core_binary', { path });
}

/**
 * Get application paths
 */
export async function getAppPaths(): Promise<AppPaths> {
    return await invoke<AppPaths>('get_app_paths');
}

/**
 * Download a profile/config from URL
 */
export async function downloadProfile(url: string): Promise<string> {
    return await invoke<string>('download_profile', { url });
}

/**
 * Get the status of the privileged helper binary (macOS only)
 */
export async function getPrivilegedHelperStatus(): Promise<boolean> {
    return await invoke<boolean>('get_privileged_helper_status');
}

/**
 * Install the privileged helper binary (macOS only)
 */
export async function installPrivilegedHelper(): Promise<void> {
    return await invoke('install_privileged_helper');
}

/**
 * Uninstall the privileged helper binary (macOS only)
 */
export async function uninstallPrivilegedHelper(): Promise<void> {
    return await invoke('uninstall_privileged_helper');
}

// ========== Core Mode Management (macOS) ==========

/**
 * Get current core mode (macOS only)
 */
export async function getCoreMode(): Promise<CoreMode> {
    return await invoke<CoreMode>('get_core_mode');
}

/**
 * Get desired core mode preference (macOS only)
 */
export async function getDesiredCoreMode(): Promise<CoreMode> {
    return await invoke<CoreMode>('get_desired_core_mode');
}

/**
 * Set desired core mode and switch if running (macOS only)
 */
export async function setCoreMode(mode: CoreMode): Promise<void> {
    return await invoke('set_core_mode', { mode });
}
