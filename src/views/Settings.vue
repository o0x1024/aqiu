<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { useMihomo } from '../composables/useMihomo';
import { useCore } from '../composables/useCore';
import { useTheme } from '../composables/useTheme';
import { getPrivilegedHelperStatus, installPrivilegedHelper, uninstallPrivilegedHelper, getCoreMode, getDesiredCoreMode, setCoreMode, downloadGeodata, type CoreMode } from '../api/tauri';
import { setUserOverride, getUserOverrides } from '../api/userOverrides';
import { useToast } from '../composables/useToast';
import { useI18n } from '../composables/useI18n';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { openPath } from '@tauri-apps/plugin-opener';
import { isMac } from '../utils/platform';

const { config, updateConfig } = useMihomo();
const {
	coreStatus,
	isLoading,
	coreExists,
	appPaths,
	restartCore,
	refreshStatus,
	setSystemProxy,
	setTunMode: setTunModeCore,
	downloadCore,
	downloadProfile,
	systemProxyEnabled,
	tunModeEnabled: tunModeEnabledCore,
	importCoreBinary,
} = useCore();
const { selectedTheme, themeOptions } = useTheme();
const { success, error, warning } = useToast();
const { t, locale, setLocale, availableLocales } = useI18n();

const coreRunning = computed(() => coreStatus.value?.running || false);
const coreVersion = computed(() => coreStatus.value?.version || t('common.unknown'));

const tunModeEnabled = computed(() => tunModeEnabledCore.value);
const controllerPort = ref<number | null>(null);
const configUrl = ref('');
const localDownloadProgress = ref<string | null>(null);
const inboundPorts = ref({
	port: config.value?.port,
	socksPort: config.value?.['socks-port'],
	mixedPort: config.value?.['mixed-port'],
	redirPort: config.value?.['redir-port'],
	tproxyPort: config.value?.['tproxy-port'],
});
const helperValid = ref(false);
const helperLoading = ref(false);
const isMacOS = isMac();
const userOverrides = ref<any>({});
const currentCoreMode = ref<CoreMode>('user');
const desiredCoreMode = ref<CoreMode>('user');
const coreModeLoading = ref(false);
const geodataLoading = ref(false);
let inboundSaveTimer: ReturnType<typeof setTimeout> | null = null;
let lastSyncedPorts = '';
let controllerDirty = false;
let syncingControllerPort = false;

const parseExternalControllerPort = (value?: string | null) => {
	if (!value) return null;
	const trimmed = value.trim();
	const parts = trimmed.split(':');
	const last = parts[parts.length - 1];
	const port = Number(last);
	return Number.isFinite(port) ? port : null;
};

const normalizePort = (value: number | null) => {
	if (!value || !Number.isFinite(value)) return null;
	const port = Math.round(value);
	if (port < 1 || port > 65535) return null;
	return port;
};

const normalizePorts = (value: typeof inboundPorts.value) => ({
	port: value.port ?? null,
	socksPort: value.socksPort ?? null,
	mixedPort: value.mixedPort ?? null,
	redirPort: value.redirPort ?? null,
	tproxyPort: value.tproxyPort ?? null,
});

const getPortsSnapshot = (value: typeof inboundPorts.value) =>
	JSON.stringify(normalizePorts(value));

const syncInboundPorts = () => {
	// Prefer user overrides, fall back to config, then default
	inboundPorts.value = {
		port: userOverrides.value?.['port'] ?? config.value?.port,
		socksPort: userOverrides.value?.['socks-port'] ?? config.value?.['socks-port'],
		mixedPort: userOverrides.value?.['mixed-port'] ?? config.value?.['mixed-port'],
		redirPort: userOverrides.value?.['redir-port'] ?? config.value?.['redir-port'],
		tproxyPort: userOverrides.value?.['tproxy-port'] ?? config.value?.['tproxy-port'],
	};
	lastSyncedPorts = getPortsSnapshot(inboundPorts.value);
};

const syncControllerPort = () => {
	if (controllerDirty) return;
	const fromOverrides = userOverrides.value?.['external-controller'];
	const fromConfig = config.value?.['external-controller'];
	const fromStatus = coreStatus.value?.api_port ? `127.0.0.1:${coreStatus.value.api_port}` : null;
	
	const controller = fromOverrides ?? fromConfig ?? fromStatus ?? '127.0.0.1:29090';
	const next = parseExternalControllerPort(controller);
	
	syncingControllerPort = true;
	controllerPort.value = next;
	syncingControllerPort = false;
};

const handleToggleSystemProxy = async () => {
	try {
		const newState = !systemProxyEnabled.value;
		const port = config.value?.['mixed-port'] || config.value?.port || 7890;
		await setSystemProxy(newState, port);
	} catch (e) {
		console.error('Failed to toggle system proxy:', e);
	}
};

const handleToggleTunMode = async () => {
	try {
		const newState = !tunModeEnabled.value;
		
		// On macOS, enabling TUN requires Service Mode
		if (isMacOS && newState) {
			// Check if we're in User Mode
			if (currentCoreMode.value === 'user' || desiredCoreMode.value === 'user') {
				// Need to switch to Service Mode for TUN
				if (!helperValid.value) {
					// Install helper first
					helperLoading.value = true;
					try {
						await installPrivilegedHelper();
						helperValid.value = await getPrivilegedHelperStatus();
						if (!helperValid.value) {
							error(t('settings.tunRequiresServiceMode') || 'TUN mode requires Service Mode. Failed to install.');
							return;
						}
					} catch (authErr) {
						console.error('Failed to install Service Mode for TUN:', authErr);
						error(t('settings.helperInstallFailed', { error: String(authErr) }));
						return;
					} finally {
						helperLoading.value = false;
					}
				}
				
				// Switch to Service Mode
				try {
					coreModeLoading.value = true;
					await setCoreMode('service');
					desiredCoreMode.value = 'service';
					currentCoreMode.value = await getCoreMode();
					success(t('settings.switchedToServiceModeForTun') || 'Switched to Service Mode for TUN mode');
				} catch (e) {
					console.error('Failed to switch to Service Mode for TUN:', e);
					error(t('settings.coreModeSwitchFailed', { error: String(e) }));
					return;
				} finally {
					coreModeLoading.value = false;
				}
			} else if (!helperValid.value) {
				// Already in Service Mode desire, but helper not installed
				helperLoading.value = true;
				try {
					await installPrivilegedHelper();
					helperValid.value = await getPrivilegedHelperStatus();
					if (!helperValid.value) {
						error(t('settings.tunRequiresHelper'));
						return;
					}
					success(t('settings.helperInstallSuccess'));
				} catch (authErr) {
					console.error('Failed to auto-authorize for TUN:', authErr);
					error(t('settings.helperInstallFailed', { error: String(authErr) }));
					return;
				} finally {
					helperLoading.value = false;
				}
			}
		}
		
		// Save to user overrides
		await setTunModeCore(newState);
		success(t('settings.tunToggleSuccess', { state: newState ? t('common.enabled') : t('common.disabled') }));
		
		// Refresh status
		await refreshStatus();
	} catch (e) {
		console.error('Failed to toggle TUN mode:', e);
		const errorMessage = e instanceof Error ? e.message : String(e);
		error(t('settings.tunToggleFailedDetail', { error: errorMessage }));
	}
};

const handleToggleAllowLan = async () => {
	if (!config.value) return;
	const newValue = !config.value['allow-lan'];
	try {
		// Save to user overrides
		await setUserOverride('allow-lan', newValue);
		// Also update runtime
		await updateConfig({ 'allow-lan': newValue });
	} catch (e) {
		error(t('settings.toggleFailed', { error: String(e) }));
	}
};

const handleDownloadCore = async () => {
	try {
		await downloadCore();
		success(t('settings.coreDownloadSuccess'));
	} catch (e) {
		console.error('Failed to download core:', e);
		error(t('settings.coreDownloadFailed', { error: String(e) }));
	}
};

const handleDownloadGeodata = async () => {
	try {
		geodataLoading.value = true;
		const result = await downloadGeodata();
		success(t('settings.geodataDownloadSuccess') || result);
	} catch (e) {
		console.error('Failed to download geodata:', e);
		error(t('settings.geodataDownloadFailed', { error: String(e) }));
	} finally {
		geodataLoading.value = false;
	}
};

const handleImportCoreBinary = async () => {
	try {
		const selected = await openDialog({
			multiple: false,
		});
		if (typeof selected !== 'string') return;
		const targetPath = await importCoreBinary(selected);
		success(t('settings.coreBinaryImported', { path: targetPath }));
	} catch (e) {
		error(t('settings.coreBinaryImportFailed', { error: String(e) }));
	}
};

const handleOpenDirectory = async (path?: string) => {
	if (!path) return;
	try {
		await openPath(path);
	} catch (e) {
		console.error('Failed to open path:', e);
		error(t('settings.openPathFailed', { error: String(e) }));
	}
};

const handleDownloadConfig = async () => {
	if (!configUrl.value) {
		warning(t('settings.configUrlRequired'));
		return;
	}
	try {
		localDownloadProgress.value = t('settings.downloadingConfig');
		await downloadProfile(configUrl.value);
		localDownloadProgress.value = null;
		success(t('settings.configSaved'));
		configUrl.value = '';
	} catch (e) {
		localDownloadProgress.value = null;
		error(t('settings.configDownloadFailed', { error: String(e) }));
	}
};

const handleSaveInboundPorts = async () => {
	try {
		const updates = [
			{ key: 'port', value: inboundPorts.value.port },
			{ key: 'socks-port', value: inboundPorts.value.socksPort },
			{ key: 'mixed-port', value: inboundPorts.value.mixedPort },
			{ key: 'redir-port', value: inboundPorts.value.redirPort },
			{ key: 'tproxy-port', value: inboundPorts.value.tproxyPort },
		];
		
		for (const update of updates) {
			await setUserOverride(update.key, update.value);
			// Update local cache
			userOverrides.value[update.key] = update.value;
		}
		
		await updateConfig({
			port: inboundPorts.value.port,
			'socks-port': inboundPorts.value.socksPort,
			'mixed-port': inboundPorts.value.mixedPort,
			'redir-port': inboundPorts.value.redirPort,
			'tproxy-port': inboundPorts.value.tproxyPort,
		});
		success(t('settings.portsSaved'));
	} catch (e) {
		error(t('settings.portsSaveFailed', { error: String(e) }));
	}
};

const handleSaveControllerPort = async () => {
	const port = normalizePort(controllerPort.value);
	if (!port) {
		warning(t('settings.apiPortInvalid'));
		return;
	}
	try {
		const controller = `127.0.0.1:${port}`;
		// Save to user overrides
		await setUserOverride('external-controller', controller);
		// Also update runtime
		await updateConfig({ 'external-controller': controller });
		controllerDirty = false;
		success(t('settings.apiPortSaved'));
	} catch (e) {
		error(t('settings.apiPortSaveFailed', { error: String(e) }));
	}
};

const handleToggleHelper = async () => {
    try {
        helperLoading.value = true;
        if (helperValid.value) {
            await uninstallPrivilegedHelper();
            helperValid.value = false;
            success(t('settings.helperUninstallSuccess') || 'Service uninstalled');
        } else {
            await installPrivilegedHelper();
            helperValid.value = await getPrivilegedHelperStatus();
            success(t('settings.helperInstallSuccess'));
        }

		// Sync core status so Dashboard start/stop immediately reflects Service Mode state
		await refreshStatus();
		
		// Refresh core mode status
		if (isMacOS) {
			try {
				currentCoreMode.value = await getCoreMode();
				desiredCoreMode.value = await getDesiredCoreMode();
			} catch (e) {
				console.error('Failed to get core mode:', e);
			}
		}
    } catch (e) {
        console.error('Failed to toggle helper:', e);
        error(t('settings.helperInstallFailed', { error: String(e) }));
    } finally {
        helperLoading.value = false;
    }
};

const handleCoreModeChange = async (mode: CoreMode) => {
	if (coreModeLoading.value) return;
	
	try {
		coreModeLoading.value = true;
		
		// If switching to service mode, check if helper is installed
		if (mode === 'service' && !helperValid.value) {
			// Auto-install helper for service mode
			try {
				await installPrivilegedHelper();
				helperValid.value = await getPrivilegedHelperStatus();
				
				if (!helperValid.value) {
					warning(t('settings.helperInstallFailed') || 'Failed to install Service Mode');
					return;
				}
			} catch (err) {
				console.error('Failed to install helper for service mode:', err);
				error(t('settings.helperInstallFailed', { error: String(err) }));
				return;
			}
		}
		
		// If switching from service mode to user mode, optionally uninstall helper
		if (mode === 'user' && currentCoreMode.value === 'service' && helperValid.value) {
			// We don't auto-uninstall, user can manually uninstall if desired
			// Just switch the mode
		}
		
		await setCoreMode(mode);
		desiredCoreMode.value = mode;
		currentCoreMode.value = await getCoreMode();
		
		success(mode === 'service' 
			? t('settings.switchedToServiceMode') || 'Switched to Service Mode'
			: t('settings.switchedToUserMode') || 'Switched to User Mode'
		);
		
		// Refresh status
		await refreshStatus();
	} catch (e) {
		console.error('Failed to switch core mode:', e);
		error(t('settings.coreModeSwitchFailed', { error: String(e) }));
		
		// Revert on error
		try {
			desiredCoreMode.value = await getDesiredCoreMode();
			currentCoreMode.value = await getCoreMode();
		} catch {}
	} finally {
		coreModeLoading.value = false;
	}
};

onMounted(async () => {
	try {
		userOverrides.value = await getUserOverrides();
        if (isMacOS) {
            helperValid.value = await getPrivilegedHelperStatus();
            try {
                currentCoreMode.value = await getCoreMode();
                desiredCoreMode.value = await getDesiredCoreMode();
            } catch (e) {
                console.error('Failed to get core mode:', e);
            }
        }
	} catch (e) {
		console.error('Failed to get status:', e);
	}

	syncInboundPorts();
	syncControllerPort();
});

watch(config, () => {
	syncInboundPorts();
	syncControllerPort();
});

watch(coreStatus, () => {
	syncControllerPort();
});

watch(controllerPort, () => {
	if (!syncingControllerPort) {
		controllerDirty = true;
	}
});

import { useStorage } from '@vueuse/core';
const showTraySpeed = useStorage('aqiu-show-tray-speed', false);

const showAppearance = ref(false);
const showAdvancedPorts = ref(false);

watch(
	inboundPorts,
	() => {
		const snapshot = getPortsSnapshot(inboundPorts.value);
		if (snapshot === lastSyncedPorts) return;
		if (inboundSaveTimer) {
			clearTimeout(inboundSaveTimer);
		}
		inboundSaveTimer = setTimeout(() => {
			handleSaveInboundPorts();
			lastSyncedPorts = snapshot;
		}, 1000);
	},
	{ deep: true }
);

let controllerSaveTimer: ReturnType<typeof setTimeout> | null = null;
watch(controllerPort, (newPort) => {
	if (syncingControllerPort) return;
	
    // Prevent false positives by comparing to actual running state
    const currentRunning = coreStatus.value?.api_port;
    if (newPort === currentRunning) {
        // If the new value matches what's currently running, no need to save/restart
        return;
    }

	controllerDirty = true;
	if (controllerSaveTimer) clearTimeout(controllerSaveTimer);
	controllerSaveTimer = setTimeout(() => {
		if (newPort) handleSaveControllerPort();
	}, 1000);
});
</script>

<template>
	<div class="space-y-8 animate-in slide-in-from-bottom-4 duration-500 max-w-6xl mx-auto">
		<div class="flex flex-col gap-2 sm:flex-row sm:items-end sm:justify-between">
			<h1 class="text-3xl font-bold tracking-tight">{{ t('settings.title') }}</h1>
			<div class="flex items-center gap-2 text-[10px] uppercase tracking-wider opacity-50">
				<span>{{ t('settings.coreSection') }}</span>
				<span>•</span>
				<span>{{ t('settings.networkSection') }}</span>
			</div>
		</div>

		<div class="grid grid-cols-1 xl:grid-cols-3 gap-6">
			<div class="space-y-6 xl:col-span-2">
				<!-- Core Section -->
				<div class="card bg-base-200 shadow-sm border border-white/5">
					<div class="card-body">
						<h3 class="card-title text-sm font-bold uppercase opacity-60 mb-4">{{ t('settings.coreSection') }}</h3>
						<div class="space-y-4">
							<div class="flex items-center justify-between">
								<div>
									<div class="font-medium">{{ t('settings.coreStatus') }}</div>
									<div class="text-xs opacity-50">{{ t('settings.coreStatusHint') }}</div>
								</div>
								<div class="flex items-center gap-3">
									<div class="badge" :class="coreRunning ? 'badge-success' : 'badge-ghost'">
										{{ coreRunning ? t('settings.running') : t('settings.stopped') }}
									</div>
									<button class="btn btn-sm btn-ghost bg-base-300" @click="restartCore"
										:disabled="isLoading">
										<i class="fa-solid fa-rotate"></i>
									</button>
								</div>
							</div>
							<div class="divider my-0 opacity-10"></div>
							<div class="flex items-center justify-between">
								<div>
									<div class="font-medium">{{ t('settings.coreVersion') }}</div>
									<div class="text-xs opacity-50">{{ t('settings.coreVersionHint') }}</div>
								</div>
								<div class="font-mono text-xs">{{ coreVersion }}</div>
							</div>
							<div class="divider my-0 opacity-10"></div>
							<div class="flex items-center justify-between gap-4">
								<div>
									<div class="font-medium">{{ t('settings.apiPort') }}</div>
									<div class="text-xs opacity-50">{{ t('settings.apiPortHint') }}</div>
								</div>
								<div class="flex items-center gap-2">
									<input type="number" v-model.number="controllerPort"
										class="input input-bordered input-sm w-24 bg-base-200 border-none focus:outline-none" />
								</div>
							</div>
							<div class="divider my-0 opacity-10"></div>
							<div class="flex items-center justify-between gap-4">
								<div>
									<div class="font-medium">{{ t('settings.coreBinary') }}</div>
									<div class="text-xs opacity-50">{{ t('settings.coreBinaryHint') }}</div>
								</div>
								<div class="flex flex-wrap items-center gap-2">
									<button class="btn btn-xs btn-ghost bg-base-300" @click="handleImportCoreBinary"
										:disabled="isLoading">{{ t('settings.coreBinarySelect') }}</button>
									<button v-if="!coreExists" class="btn btn-xs btn-primary" @click="handleDownloadCore"
										:disabled="isLoading">{{ t('actions.download') }}</button>
									<div v-else class="badge badge-success badge-outline badge-sm">{{ t('settings.coreFound') }}</div>
								</div>
							</div>
							<div class="bg-base-300/60 p-2 rounded text-[10px] font-mono break-all opacity-70">
								{{ appPaths?.mihomo_binary || '--' }}
							</div>
							<div class="divider my-0 opacity-10"></div>
							<div class="flex items-center justify-between gap-4">
								<div>
									<div class="font-medium">{{ t('settings.geodata') || 'GeoIP Database' }}</div>
									<div class="text-xs opacity-50">{{ t('settings.geodataHint') || 'Download GeoIP and GeoSite for DNS rules' }}</div>
								</div>
								<button class="btn btn-xs btn-ghost bg-base-300" @click="handleDownloadGeodata"
									:disabled="geodataLoading">
									<span v-if="geodataLoading" class="loading loading-spinner loading-xs"></span>
									<i v-else class="fa-solid fa-globe"></i>
									{{ t('settings.syncGeodata') || 'Sync' }}
								</button>
							</div>
						</div>
					</div>
				</div>

				<!-- Network Section -->
				<div class="card bg-base-200 shadow-sm border border-white/5">
					<div class="card-body">
						<h3 class="card-title text-sm font-bold uppercase opacity-60 mb-4">{{ t('settings.networkSection') }}</h3>
						<div class="space-y-4">
							<div class="flex items-center justify-between">
								<div>
									<div class="font-medium">{{ t('settings.systemProxy') }}</div>
									<div class="text-xs opacity-50">{{ t('settings.systemProxyHint') }}</div>
								</div>
								<input type="checkbox" class="toggle toggle-primary" :checked="systemProxyEnabled"
									@change="handleToggleSystemProxy" />
							</div>
							<div class="divider my-0 opacity-10"></div>
							<div class="flex items-center justify-between">
								<div>
									<div class="font-medium">{{ t('settings.tunMode') }}</div>
									<div class="text-xs opacity-50">{{ t('settings.tunModeHint') }}</div>
								</div>
								<input type="checkbox" class="toggle toggle-primary" :checked="tunModeEnabled"
									@change="handleToggleTunMode" />
							</div>
							
							<!-- Core Mode Selection (macOS only) -->
							<div v-if="isMacOS" class="divider my-0 opacity-10"></div>
							<div v-if="isMacOS" class="flex flex-col gap-3">
								<div>
									<div class="font-medium">{{ t('settings.coreMode') || 'Core Mode' }}</div>
									<div class="text-xs opacity-50">{{ t('settings.coreModeHint') || 'Choose how Mihomo runs on your system' }}</div>
								</div>
								<div class="grid grid-cols-2 gap-2">
									<button 
										class="btn btn-sm justify-start" 
										:class="desiredCoreMode === 'user' ? 'btn-primary' : 'btn-ghost bg-base-300'"
										@click="handleCoreModeChange('user')"
										:disabled="coreModeLoading">
										<i class="fa-solid fa-user"></i>
										{{ t('settings.userMode') || 'User Mode' }}
										<div v-if="currentCoreMode === 'user' && desiredCoreMode === 'user' && !coreModeLoading" class="badge badge-xs badge-success">{{ t('common.active') || 'Active' }}</div>
										<span v-if="coreModeLoading && desiredCoreMode === 'user'" class="loading loading-spinner loading-xs"></span>
									</button>
									<button 
										class="btn btn-sm justify-between w-full group" 
										:class="desiredCoreMode === 'service' ? 'btn-primary' : 'btn-ghost bg-base-300'"
										@click="handleCoreModeChange('service')"
										:disabled="coreModeLoading">
										<div class="flex items-center gap-2">
											<i class="fa-solid fa-server"></i>
											<span>{{ t('settings.serviceMode') || 'Service Mode' }}</span>
										</div>
										<div class="flex items-center gap-1">
											<div v-if="helperValid" class="badge badge-xs badge-success">{{ t('settings.authorized') || 'Authorized' }}</div>
											<div v-if="currentCoreMode === 'service' && desiredCoreMode === 'service' && !coreModeLoading" class="badge badge-xs badge-success">{{ t('common.active') || 'Active' }}</div>
											<span v-if="coreModeLoading && desiredCoreMode === 'service'" class="loading loading-spinner loading-xs"></span>
											<button 
												v-if="helperValid && !coreModeLoading"
												class="btn btn-xs btn-circle ml-1 opacity-0 group-hover:opacity-100 transition-all"
												:class="desiredCoreMode === 'service' ? 'bg-white/20 hover:bg-white hover:text-error' : 'btn-ghost hover:bg-error hover:text-error-content'"
												@click.stop="handleToggleHelper()"
												:disabled="helperLoading"
												:title="t('settings.uninstall') || 'Uninstall'">
												<i class="fa-solid fa-trash text-xs"></i>
											</button>
										</div>
									</button>
								</div>
								<div class="text-[10px] opacity-60 bg-base-300/40 rounded p-2">
									<div><strong>{{ t('settings.userMode') || 'User Mode' }}:</strong> {{ t('settings.userModeDesc') || 'Runs in your user session. TUN mode is not recommended (requires password each time)' }}</div>
									<div class="mt-1"><strong>{{ t('settings.serviceMode') || 'Service Mode' }}:</strong> {{ t('settings.serviceModeDesc') || 'Runs as system service with root privileges. Enables TUN mode without password prompts. Click to auto-install, right-click or use trash icon to uninstall.' }}</div>
								</div>
							</div>
							
							<div class="divider my-0 opacity-10"></div>
							<div class="flex items-center justify-between">
								<div>
									<div class="font-medium">{{ t('settings.allowLan') }}</div>
									<div class="text-xs opacity-50">{{ t('settings.allowLanHint') }}</div>
								</div>
								<input type="checkbox" class="toggle toggle-primary" :checked="config?.['allow-lan']"
									@change="handleToggleAllowLan" />
							</div>
							<div class="divider my-0 opacity-10"></div>
							<div class="flex items-center justify-between">
								<div>
									<div class="font-medium">{{ t('settings.showTraySpeed') }}</div>
									<div class="text-xs opacity-50">{{ t('settings.showTraySpeedHint') }}</div>
								</div>
								<input type="checkbox" class="toggle toggle-primary" v-model="showTraySpeed" />
							</div>
						</div>
					</div>
				</div>

				<!-- Inbound Ports Section -->
				<div class="card bg-base-200 shadow-sm border border-white/5">
					<div class="card-body">
						<h3 class="card-title text-sm font-bold uppercase opacity-60 mb-4">{{ t('settings.inboundPorts') }}</h3>
						<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
							<div class="form-control w-full">
								<label class="label"><span class="label-text text-xs">{{ t('settings.portMixed') }}</span></label>
								<input type="number" v-model.number="inboundPorts.mixedPort"
									class="input input-bordered input-sm bg-base-200 border-none focus:outline-none" />
							</div>
							<div class="flex items-end mb-1">
								<button class="btn btn-xs btn-ghost gap-1 opacity-50 hover:opacity-100" @click="showAdvancedPorts = !showAdvancedPorts">
									<i class="fa-solid" :class="showAdvancedPorts ? 'fa-chevron-up' : 'fa-chevron-down'"></i>
									{{ showAdvancedPorts ? t('actions.hideAdvanced') || 'Hide Advanced' : t('actions.showAdvanced') || 'Show Advanced' }}
								</button>
							</div>
						</div>

						<div v-if="showAdvancedPorts" class="grid grid-cols-1 md:grid-cols-2 gap-4 mt-4 animate-in slide-in-from-top-2 duration-200">
							<div class="form-control w-full">
								<label class="label"><span class="label-text text-xs">{{ t('settings.portHttp') }}</span></label>
								<input type="number" v-model.number="inboundPorts.port"
									class="input input-bordered input-sm bg-base-200 border-none focus:outline-none" />
							</div>
							<div class="form-control w-full">
								<label class="label"><span class="label-text text-xs">{{ t('settings.portSocks') }}</span></label>
								<input type="number" v-model.number="inboundPorts.socksPort"
									class="input input-bordered input-sm bg-base-200 border-none focus:outline-none" />
							</div>
							<div class="form-control w-full">
								<label class="label"><span class="label-text text-xs">{{ t('settings.portRedir') }}</span></label>
								<input type="number" v-model.number="inboundPorts.redirPort"
									class="input input-bordered input-sm bg-base-200 border-none focus:outline-none" />
							</div>
							<div class="form-control w-full">
								<label class="label"><span class="label-text text-xs">{{ t('settings.portTproxy') }}</span></label>
								<input type="number" v-model.number="inboundPorts.tproxyPort"
									class="input input-bordered input-sm bg-base-200 border-none focus:outline-none" />
							</div>
						</div>

					</div>
				</div>

				<!-- Quick Import Section -->
				<div class="card bg-base-200 shadow-sm border border-white/5">
					<div class="card-body">
						<h3 class="card-title text-sm font-bold uppercase opacity-60 mb-4">{{ t('settings.quickImport') }}</h3>
						<div class="space-y-4">
							<div class="form-control w-full">
								<label class="label">
									<span class="label-text text-xs opacity-50">{{ t('settings.subscriptionUrl') }}</span>
								</label>
								<div class="join w-full">
									<input type="url" v-model="configUrl" :placeholder="t('settings.importHint')"
										class="input input-bordered join-item w-full bg-base-300 border-none focus:outline-none text-sm" />
									<button class="btn btn-primary join-item" @click="handleDownloadConfig">{{ t('actions.import') }}</button>
								</div>
								<div v-if="localDownloadProgress" class="text-xs text-info mt-2">
									{{ localDownloadProgress }}
								</div>
							</div>
						</div>
					</div>
				</div>
			</div>

			<div class="space-y-6">
				<!-- Appearance Section -->
				<div class="card bg-base-200 shadow-sm border border-white/5">
					<div class="card-body">
						<div class="flex items-center justify-between cursor-pointer group" @click="showAppearance = !showAppearance">
							<h3 class="card-title text-sm font-bold uppercase opacity-60">{{ t('settings.appearance') }}</h3>
							<i class="fa-solid transition-transform duration-200 opacity-40 group-hover:opacity-100" 
								:class="showAppearance ? 'fa-chevron-up' : 'fa-chevron-down'"></i>
						</div>
						
						<div v-if="showAppearance" class="space-y-4 mt-4 animate-in slide-in-from-top-2 duration-200">
							<div class="flex items-center justify-between">
								<div>
									<div class="font-medium">{{ t('settings.theme') }}</div>
									<div class="text-xs opacity-50">{{ t('settings.themeHint') }}</div>
								</div>
								<div class="badge badge-ghost capitalize">{{ selectedTheme }}</div>
							</div>
							<div class="divider my-0 opacity-10"></div>
							<div class="grid grid-cols-2 gap-3">
								<button v-for="theme in themeOptions" :key="theme" type="button"
									class="btn btn-ghost h-auto justify-start p-3 border border-base-300/60"
									:class="selectedTheme === theme ? 'ring-2 ring-primary/60 ring-offset-2 ring-offset-base-200' : ''"
									@click="selectedTheme = theme">
									<div class="flex items-center gap-3 w-full">
										<div class="w-12 h-10 rounded-lg border border-base-300 bg-base-100 p-1"
											:data-theme="theme">
											<div class="flex h-full items-center justify-between">
												<span class="h-2.5 w-2.5 rounded-full bg-primary"></span>
												<span class="h-2.5 w-2.5 rounded-full bg-secondary"></span>
												<span class="h-2.5 w-2.5 rounded-full bg-accent"></span>
												<span class="h-2.5 w-2.5 rounded-full bg-base-300"></span>
											</div>
										</div>
										<div class="text-left">
											<div class="font-semibold capitalize leading-tight">{{ theme }}</div>
										</div>
									</div>
								</button>
							</div>
						</div>
					</div>
				</div>

				<!-- Language Section -->
				<div class="card bg-base-200 shadow-sm border border-white/5">
					<div class="card-body">
						<h3 class="card-title text-sm font-bold uppercase opacity-60 mb-4">{{ t('settings.language') }}</h3>
						<div class="form-control w-full">
							<label class="label">
								<span class="label-text text-xs opacity-50">{{ t('settings.language') }}</span>
							</label>
							<select class="select select-bordered bg-base-300 border-none focus:outline-none"
								:value="locale" @change="setLocale(($event.target as HTMLSelectElement).value as 'en' | 'zh')">
								<option v-for="item in availableLocales" :key="item.value" :value="item.value">
									{{ item.label }}
								</option>
							</select>
						</div>
					</div>
				</div>

				<!-- Paths Section -->
				<div class="card bg-base-200 shadow-sm border border-white/5" v-if="appPaths">
					<div class="card-body">
						<h3 class="card-title text-sm font-bold uppercase opacity-60 mb-4">{{ t('settings.pathsSection') }}</h3>
						<div class="space-y-4">
							<div class="flex flex-col gap-1">
								<div class="flex items-center justify-between">
									<div class="text-xs font-medium">{{ t('settings.configDir') }}</div>
									<button class="btn btn-ghost btn-xs px-2" type="button"
										@click="handleOpenDirectory(appPaths?.config_dir)">
										{{ t('settings.openPath') }}
									</button>
								</div>
								<div class="bg-base-300 p-2 rounded text-[10px] font-mono break-all opacity-70">
									{{ appPaths.config_dir }}
								</div>
							</div>
							<div class="flex flex-col gap-1">
								<div class="flex items-center justify-between">
									<div class="text-xs font-medium">{{ t('settings.dataDir') }}</div>
									<button class="btn btn-ghost btn-xs px-2" type="button"
										@click="handleOpenDirectory(appPaths?.data_dir)">
										{{ t('settings.openPath') }}
									</button>
								</div>
								<div class="bg-base-300 p-2 rounded text-[10px] font-mono break-all opacity-70">
									{{ appPaths.data_dir }}
								</div>
							</div>
						</div>
					</div>
				</div>
			</div>
		</div>
	</div>
</template>
