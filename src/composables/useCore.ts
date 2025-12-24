import { ref, onMounted, onUnmounted } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import * as tauriApi from '../api/tauri';

// Global state
const coreStatus = ref<tauriApi.CoreStatus | null>(null);
const isLoading = ref(false);
const error = ref<string | null>(null);
const coreExists = ref(false);
const appPaths = ref<tauriApi.AppPaths | null>(null);
const systemProxyEnabled = ref(false);
const tunModeEnabled = ref(false);
const proxyMode = ref('rule');
let pendingTunState: boolean | null = null;
let pendingTunPolls = 0;
const MAX_TUN_PENDING_POLLS = 3;

let statusInterval: ReturnType<typeof setInterval> | null = null;
let coreCheckInFlight = false;
let appPathsInFlight = false;
let coreStatusInFlight = false;

// Event listeners cleanup
let unlistenCoreStarted: UnlistenFn | null = null;
let unlistenCoreStopped: UnlistenFn | null = null;
let unlistenTunModeChanged: UnlistenFn | null = null;
let unlistenSystemProxyChanged: UnlistenFn | null = null;
let unlistenProxyModeChanged: UnlistenFn | null = null;
let unlistenCoreModeChanged: UnlistenFn | null = null;

export function useCore() {
    const refreshStatus = async () => {
        if (coreStatusInFlight) return;
        coreStatusInFlight = true;
        try {
            const status = await tauriApi.getCoreStatus();
			// Always fetch system proxy status, even if core is stopped.
			// System proxy is an OS-level state and can remain enabled after core stops.
			const sysProxy = await tauriApi.getSystemProxyStatus();
			systemProxyEnabled.value = sysProxy;

			// TUN status: on macOS/linux backend can fall back to config file; on Windows it may fail if core is down.
			let tun = tunModeEnabled.value;
			try {
				tun = await tauriApi.getTunStatus();
			} catch (e) {
				// best-effort: keep last known UI state
				console.warn('Failed to get TUN status', e);
			}

			if (pendingTunState !== null) {
				if (tun === pendingTunState) {
					tunModeEnabled.value = pendingTunState;
					pendingTunState = null;
					pendingTunPolls = 0;
				} else if (pendingTunPolls >= MAX_TUN_PENDING_POLLS) {
					pendingTunState = null;
					pendingTunPolls = 0;
					tunModeEnabled.value = tun;
				} else {
					pendingTunPolls += 1;
					tunModeEnabled.value = pendingTunState;
				}
			} else {
				tunModeEnabled.value = tun;
			}

			// Proxy mode is only reliably available via API when core is running.
			let mode = proxyMode.value;
			if (status.running) {
				try {
					mode = await tauriApi.getMode();
					proxyMode.value = mode;
				} catch (e) {
					console.warn('Failed to get proxy mode', e);
				}
			}

			// Update tray (reflect OS proxy even when core is stopped)
			await tauriApi.updateTrayState(sysProxy, tunModeEnabled.value, mode);
            
            coreStatus.value = status;
            error.value = null;
        } catch (e) {
            const message = String(e);
            console.error("Failed to get core status", e);
            if (!message.includes('timeout') && !message.includes('timed out')) {
                error.value = message;
            }
        } finally {
            coreStatusInFlight = false;
        }
    };

    const startCore = async (configPath?: string) => {
        isLoading.value = true;
        error.value = null;
        try {
            const options = configPath ? { config_path: configPath } : undefined;
            coreStatus.value = await tauriApi.startCore(options);
            await refreshStatus();
        } catch (e) {
            error.value = String(e);
            throw e;
        } finally {
            isLoading.value = false;
        }
    };

    const stopCore = async () => {
        isLoading.value = true;
        error.value = null;
        try {
            await tauriApi.stopCore();
            // Immediately update UI state to stopped
            if (coreStatus.value) {
                coreStatus.value = { ...coreStatus.value, running: false };
            }
            // Then refresh to get accurate state
            await refreshStatus();
        } catch (e) {
            error.value = String(e);
            throw e;
        } finally {
            isLoading.value = false;
        }
    };

    const restartCore = async () => {
        isLoading.value = true;
        error.value = null;
        try {
            coreStatus.value = await tauriApi.restartCore();
            await refreshStatus();
        } catch (e) {
            error.value = String(e);
            throw e;
        } finally {
            isLoading.value = false;
        }
    };

    const setSystemProxy = async (enable: boolean, port?: number) => {
        isLoading.value = true;
        error.value = null;
        try {
            await tauriApi.setSystemProxy(enable, port);
            systemProxyEnabled.value = enable;
            await tauriApi.updateTrayState(enable, tunModeEnabled.value, proxyMode.value);
        } catch (e) {
            error.value = String(e);
            throw e;
        } finally {
            isLoading.value = false;
        }
    };

    const setTunMode = async (enable: boolean) => {
        const previousState = tunModeEnabled.value;
        isLoading.value = true;
        error.value = null;
        try {
            pendingTunState = enable;
            pendingTunPolls = 0;
            tunModeEnabled.value = enable;
            await tauriApi.setTunMode(enable);
            // Immediately set UI state
            await tauriApi.updateTrayState(systemProxyEnabled.value, enable, proxyMode.value);
            // Refresh status to sync other states, but preserve TUN state we just set
            await refreshStatus();
        } catch (e) {
            pendingTunState = null;
            pendingTunPolls = 0;
            tunModeEnabled.value = previousState;
            error.value = String(e);
            throw e;
        } finally {
            isLoading.value = false;
        }
    }

    const setMode = async (mode: string) => {
        isLoading.value = true;
        error.value = null;
        try {
            await tauriApi.setMode(mode);
            proxyMode.value = mode;
            await tauriApi.updateTrayState(systemProxyEnabled.value, tunModeEnabled.value, mode);
        } catch (e) {
            error.value = String(e);
            throw e;
        } finally {
            isLoading.value = false;
        }
    }

    const checkCore = async () => {
        if (coreCheckInFlight) return;
        coreCheckInFlight = true;
        try {
            coreExists.value = await tauriApi.checkCoreExists();
        } catch (e) {
            console.error("Failed to check core", e);
            coreExists.value = false;
        } finally {
            coreCheckInFlight = false;
        }
    };

    const loadPaths = async () => {
        if (appPathsInFlight) return;
        appPathsInFlight = true;
        try {
            appPaths.value = await tauriApi.getAppPaths();
        } catch (e) {
            console.error("Failed to get app paths", e);
        } finally {
            appPathsInFlight = false;
        }
    };

    const setupEventListeners = async () => {
        // Listen for core started event
        unlistenCoreStarted = await listen('core-started', async (event: any) => {
            console.log('Core started event:', event.payload);
            if (event.payload.success) {
                await refreshStatus();
            } else {
                error.value = event.payload.message || 'Failed to start core';
            }
        });

        // Listen for core stopped event
        unlistenCoreStopped = await listen('core-stopped', async (event: any) => {
            console.log('Core stopped event:', event.payload);
            if (coreStatus.value) {
                coreStatus.value = { ...coreStatus.value, running: false };
            }
            await refreshStatus();
        });

        // Listen for TUN mode changed event
        unlistenTunModeChanged = await listen('tun-mode-changed', async (event: any) => {
            console.log('TUN mode changed event:', event.payload);
            tunModeEnabled.value = event.payload.enabled;
            await tauriApi.updateTrayState(systemProxyEnabled.value, event.payload.enabled, proxyMode.value);
            await refreshStatus();
        });

        // Listen for system proxy changed event
        unlistenSystemProxyChanged = await listen('system-proxy-changed', async (event: any) => {
            console.log('System proxy changed event:', event.payload);
            systemProxyEnabled.value = event.payload.enabled;
            await tauriApi.updateTrayState(event.payload.enabled, tunModeEnabled.value, proxyMode.value);
        });

        // Listen for proxy mode changed event
        unlistenProxyModeChanged = await listen('proxy-mode-changed', async (event: any) => {
            console.log('Proxy mode changed event:', event.payload);
            proxyMode.value = event.payload.mode;
            await tauriApi.updateTrayState(systemProxyEnabled.value, tunModeEnabled.value, event.payload.mode);
        });

        // Listen for core mode changed event
        unlistenCoreModeChanged = await listen('core-mode-changed', async (event: any) => {
            console.log('Core mode changed event:', event.payload);
            await refreshStatus();
        });
    };

    const cleanupEventListeners = () => {
        if (unlistenCoreStarted) unlistenCoreStarted();
        if (unlistenCoreStopped) unlistenCoreStopped();
        if (unlistenTunModeChanged) unlistenTunModeChanged();
        if (unlistenSystemProxyChanged) unlistenSystemProxyChanged();
        if (unlistenProxyModeChanged) unlistenProxyModeChanged();
        if (unlistenCoreModeChanged) unlistenCoreModeChanged();
    };

    onMounted(async () => {
        // Setup event listeners first
        await setupEventListeners();

        // Initial load
        if (!coreExists.value) checkCore();
        if (!appPaths.value) loadPaths();
        await refreshStatus();

        // Reduced polling (30s) as fallback - events are primary update mechanism
        if (!statusInterval) {
            statusInterval = setInterval(refreshStatus, 30000);
        }
    });

    onUnmounted(() => {
        // Cleanup event listeners
        cleanupEventListeners();
        
        // Stop polling
        if (statusInterval) {
            clearInterval(statusInterval);
            statusInterval = null;
        }
    });

    // Do not clear interval on unmount for singleton behavior

    return {
        coreStatus,
        isLoading,
        error,
        coreExists,
        appPaths,
        startCore,
        stopCore,
        restartCore,
        setSystemProxy,
        setTunMode,
        setMode,
        refreshStatus,
        checkCore,
        downloadCore: async (version?: string) => {
            isLoading.value = true;
            error.value = null;
            try {
                await tauriApi.downloadCore(version);
                await checkCore();
                await loadPaths();
            } catch (e) {
                error.value = String(e);
                throw e;
            } finally {
                isLoading.value = false;
            }
        },
        downloadProfile: async (url: string) => {
            isLoading.value = true;
            error.value = null;
            try {
                return await tauriApi.downloadProfile(url);
            } catch (e) {
                error.value = String(e);
                throw e;
            } finally {
                isLoading.value = false;
            }
        },
        importCoreBinary: async (path: string) => {
            isLoading.value = true;
            error.value = null;
            try {
                const targetPath = await tauriApi.importCoreBinary(path);
                await checkCore();
                await loadPaths();
                return targetPath;
            } catch (e) {
                error.value = String(e);
                throw e;
            } finally {
                isLoading.value = false;
            }
        },
        getSystemProxyStatus: tauriApi.getSystemProxyStatus,
        systemProxyEnabled,
        tunModeEnabled,
        getTunStatus: tauriApi.getTunStatus,
        proxyMode,
    };
}
