<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { useCore } from './composables/useCore';
import { useTheme } from './composables/useTheme';
import { useI18n } from './composables/useI18n';
import { useToast } from './composables/useToast';
import { useTray } from './composables/useTray';
import { useMihomo } from './composables/useMihomo';

// Views
import Dashboard from './views/Dashboard.vue';
import Proxies from './views/Proxies.vue';
import Rules from './views/Rules.vue';
import Connections from './views/Connections.vue';
import Logs from './views/Logs.vue';
import Settings from './views/Settings.vue';
import ProfilesManager from './components/ProfilesManager.vue';
import ToastContainer from './components/ToastContainer.vue';
import DialogContainer from './components/DialogContainer.vue';

const currentView = ref('dashboard');
const downloadProgress = ref<string | null>(null);

// Floating panel drag state
const floatingPanelPos = ref({ x: 0, y: 50 }); // percentage from top-right
const isDragging = ref(false);
const dragStart = ref({ x: 0, y: 0 });
const panelHidden = ref(false);
let hideTimeout: ReturnType<typeof setTimeout> | null = null;

const {
	coreStatus,
	coreExists,
	isLoading,
	downloadCore,
	restartCore,
	refreshStatus,
	systemProxyEnabled,
	setSystemProxy,
	tunModeEnabled,
	setTunMode,
} = useCore();
const { fetchProxies, config, updateConfig } = useMihomo();

// Computed for floating controls
const coreRunning = computed(() => coreStatus.value?.running || false);
const showFloatingControls = computed(() => currentView.value !== 'dashboard');
const currentMode = computed(() => {
	const raw = (config.value?.mode || 'rule').toLowerCase();
	switch (raw) {
		case 'global': return 'global';
		case 'direct': return 'direct';
		default: return 'rule';
	}
});

const handleToggleSystemProxy = async () => {
	try {
		const newState = !systemProxyEnabled.value;
		const port = config.value?.['mixed-port'] || config.value?.port || 7890;
		await setSystemProxy(newState, port);
	} catch (e) {
		console.error('Failed to toggle system proxy:', e);
		error(t('settings.toggleFailed', { error: String(e) }));
	}
};

const handleToggleTunMode = async () => {
	try {
		const newState = !tunModeEnabled.value;
		await setTunMode(newState);
	} catch (e) {
		console.error('Failed to toggle TUN mode:', e);
		error(t('settings.tunToggleFailed', { error: String(e) }));
	}
};

// Floating panel drag handlers
const startDrag = (e: MouseEvent) => {
	e.preventDefault(); // Prevent text selection
	e.stopPropagation();
	isDragging.value = true;
	dragStart.value = { x: e.clientX, y: e.clientY };
	document.addEventListener('mousemove', onDrag);
	document.addEventListener('mouseup', stopDrag);
	// Prevent text selection during drag
	document.body.style.userSelect = 'none';
};

const onDrag = (e: MouseEvent) => {
	if (!isDragging.value) return;
	e.preventDefault();
	const dx = dragStart.value.x - e.clientX;
	const dy = e.clientY - dragStart.value.y;
	floatingPanelPos.value = {
		x: Math.max(0, Math.min(90, floatingPanelPos.value.x + (dx / window.innerWidth) * 100)),
		y: Math.max(10, Math.min(90, floatingPanelPos.value.y + (dy / window.innerHeight) * 100))
	};
	dragStart.value = { x: e.clientX, y: e.clientY };
	// Save position to localStorage
	localStorage.setItem('floatingPanelPos', JSON.stringify(floatingPanelPos.value));
};

const stopDrag = () => {
	isDragging.value = false;
	document.removeEventListener('mousemove', onDrag);
	document.removeEventListener('mouseup', stopDrag);
	// Restore text selection
	document.body.style.userSelect = '';
};

// Auto-hide to edge
const showPanel = () => {
	panelHidden.value = false;
	if (hideTimeout) {
		clearTimeout(hideTimeout);
		hideTimeout = null;
	}
};

const scheduleHide = () => {
	if (hideTimeout) clearTimeout(hideTimeout);
	hideTimeout = setTimeout(() => {
		// Only hide if close to edge (x < 5%)
		if (floatingPanelPos.value.x < 5) {
			panelHidden.value = true;
		}
	}, 2000); // Hide after 2 seconds of no interaction
};

const onPanelMouseEnter = () => {
	showPanel();
};

const onPanelMouseLeave = () => {
	if (!isDragging.value) {
		scheduleHide();
	}
};

const { setMode } = useCore();
const handleChangeMode = async (mode: string) => {
	try {
		await setMode(mode);
		await updateConfig({ mode });
	} catch (e) {
		console.error('Failed to change mode:', e);
		error(t('dashboard.modeUpdateFailed', { error: String(e) }));
	}
};

const { selectedTheme, applyTheme } = useTheme();
const { t } = useI18n();
const { error } = useToast();

useTray();

// View mapping
const views: Record<string, any> = {
	dashboard: Dashboard,
	proxies: Proxies,
	rules: Rules,
	connections: Connections,
	logs: Logs,
	settings: Settings,
	profiles: ProfilesManager
};

const currentViewComponent = computed(() => views[currentView.value]);

const switchView = (view: string) => currentView.value = view;

const handleDownloadCore = async () => {
	try {
		downloadProgress.value = t('core.downloadStarting');
		await downloadCore();
		downloadProgress.value = null;
	} catch (e) {
		console.error('Failed to download core:', e);
		downloadProgress.value = t('core.downloadFailed', { error: String(e) });
	}
};

// Listen for events
let unlisten: (() => void) | null = null;

onMounted(async () => {
	await refreshStatus();
	unlisten = await listen<string>('download-progress', (event) => {
		// Clear progress if empty string is received
		downloadProgress.value = event.payload || null;
	});

	applyTheme(selectedTheme.value);
	
	// Restore floating panel position from localStorage
	const savedPos = localStorage.getItem('floatingPanelPos');
	if (savedPos) {
		try {
			const pos = JSON.parse(savedPos);
			// Validate position is within bounds
			floatingPanelPos.value = {
				x: Math.max(0, Math.min(90, pos.x || 0)),
				y: Math.max(10, Math.min(90, pos.y || 50))
			};
		} catch (e) {
			console.error('Failed to parse saved panel position:', e);
		}
	}
})

onUnmounted(() => {
	if (unlisten) unlisten();
});

// Handle events from child components
const handleApplyProfile = async () => {
	try {
		await restartCore();
		
		// Poll for core status and API readiness
		let ready = false;
		for (let i = 0; i < 60; i++) { // Wait up to 30 seconds
			await refreshStatus();
			if (!coreStatus.value?.running) {
				throw new Error('Core process exited unexpectedly');
			}

			// Try to fetch proxies to verify API is responsive
			// We use a short timeout/retry here because we are polling
			if (await fetchProxies(1, 200)) {
				ready = true;
				break;
			}
			await new Promise(resolve => setTimeout(resolve, 500));
		}

		if (!ready) {
			throw new Error('Core started but API is unreachable');
		}
	} catch (e) {
		console.error('Failed to apply profile:', e);
		error(t('profiles.activateFailed', { error: String(e) }));
	}
};

</script>

<template>
	<div class="drawer lg:drawer-open h-screen">
		<input id="app-drawer" type="checkbox" class="drawer-toggle" />

		<!-- Main Content -->
		<div class="drawer-content flex flex-col overflow-hidden">
			<!-- Top Navbar (Mobile only) -->
			<div class="navbar bg-base-200 lg:hidden border-b border-white/5">
				<div class="flex-1">
					<a class="btn btn-ghost text-xl gap-2">
						<img src="/icon.png" alt="Aqiu" class="w-6 h-6 rounded" />
						{{ t('app.title') }}
					</a>
				</div>
				<div class="flex-none">
					<ul class="menu menu-horizontal gap-2 flex-nowrap px-2 py-2">
						<li>
							<a @click="switchView('dashboard')"
								:class="{ 'active bg-primary text-white shadow-md shadow-primary/20': currentView === 'dashboard' }"
								class="gap-2">
								<i class="fa-solid fa-gauge-high"></i>
								<span class="text-xs">{{ t('nav.dashboard') }}</span>
							</a>
						</li>
						<li>
							<a @click="switchView('proxies')"
								:class="{ 'active bg-primary text-white shadow-md shadow-primary/20': currentView === 'proxies' }"
								class="gap-2">
								<i class="fa-solid fa-server"></i>
								<span class="text-xs">{{ t('nav.proxies') }}</span>
							</a>
						</li>
						<li>
							<a @click="switchView('rules')"
								:class="{ 'active bg-primary text-white shadow-md shadow-primary/20': currentView === 'rules' }"
								class="gap-2">
								<i class="fa-solid fa-route"></i>
								<span class="text-xs">{{ t('nav.rules') }}</span>
							</a>
						</li>
						<li>
							<a @click="switchView('profiles')"
								:class="{ 'active bg-primary text-white shadow-md shadow-primary/20': currentView === 'profiles' }"
								class="gap-2">
								<i class="fa-solid fa-folder-open"></i>
								<span class="text-xs">{{ t('nav.profiles') }}</span>
							</a>
						</li>
						<li>
							<a @click="switchView('connections')"
								:class="{ 'active bg-primary text-white shadow-md shadow-primary/20': currentView === 'connections' }"
								class="gap-2">
								<i class="fa-solid fa-link"></i>
								<span class="text-xs">{{ t('nav.connections') }}</span>
							</a>
						</li>
						<li>
							<a @click="switchView('logs')"
								:class="{ 'active bg-primary text-white shadow-md shadow-primary/20': currentView === 'logs' }"
								class="gap-2">
								<i class="fa-solid fa-scroll"></i>
								<span class="text-xs">{{ t('nav.logs') }}</span>
							</a>
						</li>
						<li>
							<a @click="switchView('settings')"
								:class="{ 'active bg-primary text-white shadow-md shadow-primary/20': currentView === 'settings' }"
								class="gap-2">
								<i class="fa-solid fa-gear"></i>
								<span class="text-xs">{{ t('nav.settings') }}</span>
							</a>
						</li>
					</ul>
				</div>
			</div>

			<!-- Main View Area -->
			<main class="flex-1 overflow-y-auto p-4 lg:p-8 space-y-6">
				<!-- Core Missing Alert -->
				<div v-if="!coreExists && !isLoading" class="alert alert-error shadow-lg">
					<i class="fa-solid fa-triangle-exclamation"></i>
					<div>
						<h3 class="font-bold">{{ t('core.missingTitle') }}</h3>
						<div class="text-xs">{{ t('core.missingHint') }}</div>
					</div>
					<button class="btn btn-sm btn-ghost border-white/20" @click="handleDownloadCore">
						{{ t('actions.download') }}
					</button>
				</div>

				<div v-if="downloadProgress" class="alert alert-info shadow-lg">
					<span class="loading loading-spinner loading-sm"></span>
					<span>{{ downloadProgress }}</span>
				</div>

				<!-- Dynamic View Component with KeepAlive -->
				<keep-alive>
					<component 
						:is="currentViewComponent" 
						:key="currentView"
						@apply="handleApplyProfile" 
						@back="switchView('dashboard')" 
					/>
				</keep-alive>
			</main>
		</div>

		<!-- Sidebar Drawer -->
		<div class="drawer-side z-20">
			<label for="app-drawer" class="drawer-overlay"></label>
			<div class="w-64 min-h-full bg-base-200 border-r border-white/5 flex flex-col">
				<!-- Logo -->
				<div class="p-8 flex items-center gap-3">
					<div
						class="w-10 h-10 rounded-xl flex items-center justify-center shadow-lg">
						<img src="/icon.png" alt="Aqiu" class="w-10 h-10 rounded-xl" />
					</div>
					<div class="flex flex-col">
						<span class="font-bold text-lg leading-none">{{ t('app.title') }}</span>
						<span class="text-[10px] opacity-40 uppercase tracking-widest mt-1">{{ t('app.subtitle')
						}}</span>
					</div>
				</div>

				<!-- Navigation -->
				<nav class="flex-1 px-3 py-2">
					<div class="text-[10px] uppercase tracking-widest opacity-30 font-bold px-3 mt-4 mb-2">
						{{ t('nav.main') }}
					</div>
					<a @click="switchView('dashboard')"
						:class="currentView === 'dashboard' ? 'bg-base-300 shadow-sm' : 'hover:bg-base-300/50'"
						class="flex items-center gap-3 px-3 py-3 rounded-xl cursor-pointer transition-all w-full">
						<i class="fa-solid fa-gauge-high w-5 text-center"></i>
						<span>{{ t('nav.dashboard') }}</span>
					</a>
					<a @click="switchView('proxies')"
						:class="currentView === 'proxies' ? 'bg-base-300 shadow-sm' : 'hover:bg-base-300/50'"
						class="flex items-center gap-3 px-3 py-3 rounded-xl cursor-pointer transition-all w-full">
						<i class="fa-solid fa-server w-5 text-center"></i>
						<span>{{ t('nav.proxies') }}</span>
					</a>
					<a @click="switchView('rules')"
						:class="currentView === 'rules' ? 'bg-base-300 shadow-sm' : 'hover:bg-base-300/50'"
						class="flex items-center gap-3 px-3 py-3 rounded-xl cursor-pointer transition-all w-full">
						<i class="fa-solid fa-route w-5 text-center"></i>
						<span>{{ t('nav.rules') }}</span>
					</a>
					<a @click="switchView('profiles')"
						:class="currentView === 'profiles' ? 'bg-base-300 shadow-sm' : 'hover:bg-base-300/50'"
						class="flex items-center gap-3 px-3 py-3 rounded-xl cursor-pointer transition-all w-full">
						<i class="fa-solid fa-folder-open w-5 text-center"></i>
						<span>{{ t('nav.profiles') }}</span>
					</a>

					<div class="text-[10px] uppercase tracking-widest opacity-30 font-bold px-3 mt-6 mb-2">
						{{ t('nav.network') }}
					</div>
					<a @click="switchView('connections')"
						:class="currentView === 'connections' ? 'bg-base-300 shadow-sm' : 'hover:bg-base-300/50'"
						class="flex items-center gap-3 px-3 py-3 rounded-xl cursor-pointer transition-all w-full">
						<i class="fa-solid fa-link w-5 text-center"></i>
						<span>{{ t('nav.connections') }}</span>
					</a>
					<a @click="switchView('logs')"
						:class="currentView === 'logs' ? 'bg-base-300 shadow-sm' : 'hover:bg-base-300/50'"
						class="flex items-center gap-3 px-3 py-3 rounded-xl cursor-pointer transition-all w-full">
						<i class="fa-solid fa-scroll w-5 text-center"></i>
						<span>{{ t('nav.logs') }}</span>
					</a>
				</nav>
				
				<!-- Settings at bottom -->
				<div class="px-3 pb-4">
					<a @click="switchView('settings')"
						:class="currentView === 'settings' ? 'bg-base-300 shadow-sm' : 'hover:bg-base-300/50'"
						class="flex items-center gap-3 px-3 py-3 rounded-xl cursor-pointer transition-all w-full">
						<i class="fa-solid fa-gear w-5 text-center"></i>
						<span>{{ t('nav.settings') }}</span>
					</a>
				</div>
			</div>
		</div>
	</div>

	<!-- Floating Control Panel (shown on non-Dashboard pages) -->
	<div v-if="showFloatingControls" 
		class="fixed z-50 flex flex-col gap-1 bg-base-100 rounded-2xl shadow-2xl border border-base-300 p-2 transition-transform duration-300"
		:class="{ 
			'cursor-move': !isDragging, 
			'cursor-grabbing': isDragging
		}"
		:style="{ 
			right: `${floatingPanelPos.x}%`, 
			top: `${floatingPanelPos.y}%`,
			transform: panelHidden ? 'translate(calc(100% - 16px), -50%)' : 'translate(50%, -50%)'
		}"
		@mouseenter="onPanelMouseEnter"
		@mouseleave="onPanelMouseLeave">
		
		<!-- Drag Handle -->
		<div class="w-10 h-8 rounded-t-xl flex items-center justify-center cursor-grab active:cursor-grabbing hover:bg-base-200 transition-colors"
			@mousedown="startDrag"
			title="Drag to move">
			<div class="flex gap-1">
				<div class="w-1 h-1 rounded-full bg-base-content/30"></div>
				<div class="w-1 h-1 rounded-full bg-base-content/30"></div>
				<div class="w-1 h-1 rounded-full bg-base-content/30"></div>
			</div>
		</div>
		
		<div class="divider my-0 h-px"></div>
		
		<!-- Core Status Indicator -->
		<div class="tooltip tooltip-left" :data-tip="coreRunning ? t('dashboard.coreStatusRunning') : t('dashboard.coreStatusStopped')">
			<div class="w-10 h-10 rounded-xl flex items-center justify-center"
				:class="coreRunning ? 'bg-success/20 text-success' : 'bg-error/20 text-error'">
				<span class="relative flex h-3 w-3">
					<span v-if="coreRunning" class="animate-ping absolute inline-flex h-full w-full rounded-full bg-success opacity-75"></span>
					<span class="relative inline-flex rounded-full h-3 w-3" :class="coreRunning ? 'bg-success' : 'bg-error'"></span>
				</span>
			</div>
		</div>
		
		<div class="divider my-0 h-px"></div>
		
		<!-- Mode Switcher -->
		<div class="tooltip tooltip-left" :data-tip="t('common.global')">
			<button class="w-10 h-10 rounded-xl flex items-center justify-center transition-all"
				:class="currentMode === 'global' ? 'bg-primary text-primary-content' : 'hover:bg-base-200'"
				@click="handleChangeMode('global')">
				<i class="fa-solid fa-globe text-sm"></i>
			</button>
		</div>
		<div class="tooltip tooltip-left" :data-tip="t('common.rule')">
			<button class="w-10 h-10 rounded-xl flex items-center justify-center transition-all"
				:class="currentMode === 'rule' ? 'bg-primary text-primary-content' : 'hover:bg-base-200'"
				@click="handleChangeMode('rule')">
				<i class="fa-solid fa-route text-sm"></i>
			</button>
		</div>
		<div class="tooltip tooltip-left" :data-tip="t('common.direct')">
			<button class="w-10 h-10 rounded-xl flex items-center justify-center transition-all"
				:class="currentMode === 'direct' ? 'bg-primary text-primary-content' : 'hover:bg-base-200'"
				@click="handleChangeMode('direct')">
				<i class="fa-solid fa-arrow-right text-sm"></i>
			</button>
		</div>
		
		<div class="divider my-0 h-px"></div>
		
		<!-- System Proxy Toggle -->
		<div class="tooltip tooltip-left" :data-tip="t('dashboard.proxyLabel') + ': ' + (systemProxyEnabled ? t('common.on') : t('common.off'))">
			<button class="w-10 h-10 rounded-xl flex items-center justify-center transition-all"
				:class="systemProxyEnabled ? 'bg-success text-success-content' : 'hover:bg-base-200'"
				@click="handleToggleSystemProxy">
				<svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<circle cx="12" cy="12" r="10"/>
					<path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
				</svg>
			</button>
		</div>
		
		<!-- TUN Mode Toggle -->
		<div class="tooltip tooltip-left" :data-tip="t('settings.tunMode') + ': ' + (tunModeEnabled ? t('common.on') : t('common.off'))">
			<button class="w-10 h-10 rounded-xl flex items-center justify-center transition-all"
				:class="tunModeEnabled ? 'bg-info text-info-content' : 'hover:bg-base-200'"
				@click="handleToggleTunMode">
				<svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M3 3l18 18M9.4 3.4c1.2-.9 2.6-1.4 4.1-1.4 4.4 0 8 3.6 8 8 0 1.5-.5 2.9-1.4 4.1M6 6C4.8 7.2 4 8.9 4 10.5c0 4.4 3.6 8 8 8 1.6 0 3.2-.6 4.5-1.6M9 12a3 3 0 1 0 6 0 3 3 0 0 0-6 0z"/>
				</svg>
			</button>
		</div>
		
		<div class="divider my-0 h-px"></div>
		
		<!-- Go to Dashboard -->
		<div class="tooltip tooltip-left" :data-tip="t('nav.dashboard')">
			<button class="w-10 h-10 rounded-xl flex items-center justify-center hover:bg-base-200 transition-all"
				@click="switchView('dashboard')">
				<i class="fa-solid fa-gauge-high text-sm"></i>
			</button>
		</div>
	</div>

	<!-- Toast Notifications -->
	<ToastContainer />
	<DialogContainer />
</template>
