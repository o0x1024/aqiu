<script setup lang="ts">
import { computed } from 'vue';
import { useMihomo } from '../composables/useMihomo';
import { useCore } from '../composables/useCore';
import { useI18n } from '../composables/useI18n';
import { useToast } from '../composables/useToast';

const {
	config,
	proxies,
	connections,
	traffic,
	totalTraffic,
	selectProxy,
	testLatency,
	fetchProxies,
	updateConfig,
} = useMihomo();

const {
	coreStatus,
	setSystemProxy,
	setTunMode,
	setMode,
	systemProxyEnabled,
	tunModeEnabled,
	startCore,
	stopCore,
	isLoading,
} = useCore();
const { t } = useI18n();
const { error, success } = useToast();

// Local computed properties
const coreRunning = computed(() => coreStatus.value?.running || false);
const currentMode = computed(() => {
	const raw = (config.value?.mode || 'rule').toLowerCase();
	switch (raw) {
		case 'global':
			return 'Global';
		case 'direct':
			return 'Direct';
		default:
			return 'Rule';
	}
});
const currentModeLabel = computed(() => {
	switch (currentMode.value) {
		case 'Global':
			return t('common.global');
		case 'Direct':
			return t('common.direct');
		default:
			return t('common.rule');
	}
});
const proxyGroups = computed(() => Object.values(proxies.value).filter((p) => {
	const isGroup = p.type === 'Selector' || p.type === 'URLTest';
	if (!isGroup) return false;
	if (p.name === 'GLOBAL') {
		return currentMode.value === 'Global';
	}
	return true;
}));
const modeOptions = computed(() => [
	{ value: 'Global', label: t('common.global') },
	{ value: 'Rule', label: t('common.rule') },
	{ value: 'Direct', label: t('common.direct') }
]);

// Traffic history for chart (we might want to move this to useMihomo if we want to persist it across view switches)
// For now, let's keep it here, but it will reset when switching views.
// To fix this, we should move it to useMihomo.
// But let's stick to the plan of just refactoring first.
import { ref, onMounted, onUnmounted } from 'vue';

const trafficHistory = ref<{ up: number; down: number }[]>([]);
const MAX_HISTORY = 60;
let trafficInterval: ReturnType<typeof setInterval> | null = null;

onMounted(() => {
	trafficInterval = setInterval(() => {
		trafficHistory.value.push({ up: traffic.up, down: traffic.down });
		if (trafficHistory.value.length > MAX_HISTORY) {
			trafficHistory.value.shift();
		}
	}, 1000);
});

onUnmounted(() => {
	if (trafficInterval) clearInterval(trafficInterval);
});

const handleToggleCore = async () => {
	try {
		if (coreRunning.value) {
			await stopCore();
		} else {
			await startCore();
		}
	} catch (e) {
		console.error('Failed to toggle core:', e);
		error(t('dashboard.coreToggleFailed', { error: String(e) }));
	}
};

const handleToggleSystemProxy = async () => {
	try {
		const newState = !systemProxyEnabled.value;
		// Use port from config or default to 7890
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
		success(t('settings.tunToggleSuccess', { state: newState ? t('common.enabled') : t('common.disabled') }));
	} catch (e) {
		console.error('Failed to toggle TUN mode:', e);
		error(t('settings.tunToggleFailedDetail', { error: String(e) }));
	}
};

const handleChangeMode = async (mode: string) => {
	const nextMode = mode.toLowerCase();
	try {
		await setMode(nextMode);
		// Also update via mihomo API to be sure
		await updateConfig({ mode: nextMode });
	} catch (e) {
		console.error('Failed to change mode:', e);
		error(t('dashboard.modeUpdateFailed', { error: String(e) }));
	}
};

const handleTestAllLatency = async () => {
	for (const group of proxyGroups.value) {
		for (const proxyName of group.all || []) {
			testLatency(proxyName);
		}
	}
	setTimeout(() => fetchProxies(), 3000);
};

const formatSpeed = (bytes: number) => {
	if (bytes < 1024) return `${bytes} B/s`;
	if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB/s`;
	return `${(bytes / 1024 / 1024).toFixed(1)} MB/s`;
};
const formatBytes = (bytes: number) => {
	if (bytes < 1024) return `${bytes} B`;
	if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
	if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
	return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
};

const getDelayColor = (delay: number) => {
	if (delay === 0) return '#666';
	if (delay < 200) return '#34C759';
	if (delay < 500) return '#FF9F0A';
	return '#FF3B30';
};

</script>

<template>
	<div class="space-y-8 animate-in fade-in duration-500">
		<div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
			<div class="flex flex-col">
			<h1 class="text-3xl font-bold tracking-tight">{{ t('dashboard.title') }}</h1>
			<div class="text-xs opacity-50 mt-1 flex items-center gap-2">
				{{ t('dashboard.systemIs') }}
				<span class="text-rotate font-bold text-primary">
					<span>
						<span>{{ t('dashboard.statusReady') }}</span>
						<span>{{ t('dashboard.statusOptimized') }}</span>
						<span>{{ t('dashboard.statusSecure') }}</span>
						<span>{{ t('dashboard.statusFast') }}</span>
					</span>
				</span>
			</div>
		</div>
		<div class="flex flex-wrap items-center gap-3">
			<!-- Core Status - Clickable Toggle (First Position) -->
			<button class="flex items-center gap-2 px-3 py-1.5 rounded-full border transition-all duration-200 hover:scale-105"
				:class="coreRunning ? 'border-success/30 bg-success/10 hover:bg-success/20' : 'border-error/30 bg-error/10 hover:bg-error/20'"
				@click="handleToggleCore"
				:disabled="isLoading">
				<span class="relative flex h-3 w-3">
					<span v-if="coreRunning && !isLoading" class="animate-ping absolute inline-flex h-full w-full rounded-full bg-success opacity-75"></span>
					<span v-if="isLoading" class="loading loading-spinner loading-xs"></span>
					<span v-else class="relative inline-flex rounded-full h-3 w-3" :class="coreRunning ? 'bg-success' : 'bg-error'"></span>
				</span>
				<span class="text-xs font-medium" :class="coreRunning ? 'text-success' : 'text-error'">
					{{ coreRunning ? t('common.on') : t('common.off') }}
				</span>
			</button>
			
			<!-- Mode Switcher -->
			<div class="bg-base-200 rounded-full p-1 flex shadow-sm">
				<button v-for="mode in modeOptions" :key="mode.value" 
					class="px-4 py-1.5 text-sm font-medium rounded-full transition-all duration-200"
					:class="currentMode === mode.value 
						? 'bg-primary text-primary-content shadow-md' 
						: 'text-base-content/70 hover:text-base-content'"
					@click="handleChangeMode(mode.value)">
					{{ mode.label }}
				</button>
			</div>
			
			<!-- System Proxy Button -->
			<button class="flex items-center gap-2 px-4 py-2 rounded-full font-medium text-sm transition-all duration-200 shadow-sm border"
				:class="systemProxyEnabled 
					? 'bg-success text-success-content border-success hover:bg-success/90' 
					: 'bg-base-200 text-base-content/70 border-base-300 hover:bg-base-300'"
				@click="handleToggleSystemProxy">
				<svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<circle cx="12" cy="12" r="10"/>
					<path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
				</svg>
				<span>{{ t('dashboard.proxyLabel') }}</span>
				<span class="px-2 py-0.5 rounded-full text-xs" 
					:class="systemProxyEnabled ? 'bg-white/20' : 'bg-base-300'">
					{{ systemProxyEnabled ? t('common.on') : t('common.off') }}
				</span>
			</button>
			
			<!-- TUN Mode Toggle -->
			<button class="flex items-center gap-2 px-4 py-2 rounded-full font-medium text-sm transition-all duration-200 shadow-sm border"
				:class="tunModeEnabled 
					? 'bg-info text-info-content border-info hover:bg-info/90' 
					: 'bg-base-200 text-base-content/70 border-base-300 hover:bg-base-300'"
				@click="handleToggleTunMode"
				:disabled="isLoading">
				<svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/>
				</svg>
				<span>{{ t('common.tun') }}</span>
				<span class="px-2 py-0.5 rounded-full text-xs" 
					:class="tunModeEnabled ? 'bg-white/20' : 'bg-base-300'">
					{{ tunModeEnabled ? t('common.on') : t('common.off') }}
				</span>
			</button>
		</div>
	</div>



		<!-- Stats Grid -->
		<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
			<div class="stats shadow bg-base-200 overflow-hidden">
				<div class="stat py-3 px-4">
					<div class="stat-figure text-primary">
						<i class="fa-solid fa-arrow-down text-xl"></i>
					</div>
				<div class="stat-title text-[10px] uppercase font-bold opacity-60">{{ t('dashboard.statsDownload') }}</div>
					<div class="stat-value text-lg whitespace-nowrap">{{ formatSpeed(traffic.down) }}</div>
					<div class="text-[9px] opacity-50">{{ t('dashboard.totalLabel') }} {{ formatBytes(totalTraffic.down) }}</div>
				</div>
			</div>
			<div class="stats shadow bg-base-200 overflow-hidden">
				<div class="stat py-3 px-4">
					<div class="stat-figure text-success">
						<i class="fa-solid fa-arrow-up text-xl"></i>
					</div>
				<div class="stat-title text-[10px] uppercase font-bold opacity-60">{{ t('dashboard.statsUpload') }}</div>
					<div class="stat-value text-lg whitespace-nowrap">{{ formatSpeed(traffic.up) }}</div>
					<div class="text-[9px] opacity-50">{{ t('dashboard.totalLabel') }} {{ formatBytes(totalTraffic.up) }}</div>
				</div>
			</div>
			<div class="stats shadow bg-base-200 overflow-hidden">
				<div class="stat py-3 px-4">
					<div class="stat-figure text-warning">
						<i class="fa-solid fa-link text-xl"></i>
					</div>
				<div class="stat-title text-[10px] uppercase font-bold opacity-60">{{ t('dashboard.statsConnections') }}</div>
					<div class="stat-value text-lg">{{ connections.length }}</div>
				</div>
			</div>
			<div class="stats shadow bg-base-200 overflow-hidden">
				<div class="stat py-3 px-4">
					<div class="stat-figure text-secondary">
						<i class="fa-solid fa-shuffle text-xl"></i>
					</div>
				<div class="stat-title text-[10px] uppercase font-bold opacity-60">{{ t('dashboard.statsMode') }}</div>
				<div class="stat-value text-lg">{{ currentModeLabel }}</div>
			</div>
		</div>
	</div>

		<!-- Traffic Chart Card -->
		<div class="card bg-base-200 shadow-sm border border-white/5">
			<div class="card-body p-0">
				<div class="flex items-center justify-between px-6 pt-6">
				<h3 class="card-title text-sm font-bold uppercase opacity-60">{{ t('dashboard.trafficTitle') }}</h3>
				<div class="flex gap-4 text-xs">
					<span class="flex items-center gap-1.5"><span class="w-2 h-2 rounded-full bg-primary"></span>
						{{ t('dashboard.down') }}</span>
					<span class="flex items-center gap-1.5"><span class="w-2 h-2 rounded-full bg-success"></span>
						{{ t('dashboard.up') }}</span>
				</div>
			</div>
				<div class="h-48 w-full relative mt-4">
					<svg class="w-full h-full px-4 pb-4" viewBox="0 0 600 120" preserveAspectRatio="none">
						<polyline class="stroke-primary stroke-2 transition-all duration-300" :points="(() => {
							const maxTraffic = Math.max(...trafficHistory.map(h => Math.max(h.down, h.up)), 1024 * 10);
							return trafficHistory.map((t, i) => {
								const x = (i / Math.max(trafficHistory.length - 1, 1)) * 600;
								const y = 120 - (t.down / maxTraffic) * 100;
								return `${x},${y}`;
							}).join(' ');
						})()" fill="none" />
						<polyline class="stroke-success stroke-2 transition-all duration-300" :points="(() => {
							const maxTraffic = Math.max(...trafficHistory.map(h => Math.max(h.down, h.up)), 1024 * 10);
							return trafficHistory.map((t, i) => {
								const x = (i / Math.max(trafficHistory.length - 1, 1)) * 600;
								const y = 120 - (t.up / maxTraffic) * 100;
								return `${x},${y}`;
							}).join(' ');
						})()" fill="none" />
					</svg>
					<div v-if="trafficHistory.length < 2"
						class="absolute inset-0 flex items-center justify-center bg-base-200/50 backdrop-blur-sm rounded-b-2xl">
						<span class="text-sm opacity-50">{{ t('dashboard.collecting') }}</span>
					</div>
				</div>
			</div>
		</div>

		<!-- Quick Proxy Selector -->
		<div class="card bg-base-200 shadow-sm border border-white/5" v-if="proxyGroups.length > 0">
			<div class="card-body">
				<div class="flex items-center justify-between mb-4">
					<h3 class="card-title text-sm font-bold uppercase opacity-60">{{ proxyGroups[0].name }}
					</h3>
					<button class="btn btn-xs btn-ghost" @click="handleTestAllLatency">
						<i class="fa-solid fa-bolt"></i> {{ t('actions.testAll') }}
					</button>
				</div>
				<div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-2">
					<button v-for="proxyName in (proxyGroups[0].all || []).slice(0, 12)" :key="proxyName"
						class="btn btn-sm h-auto py-2 flex flex-col items-start gap-0.5"
						:class="proxyGroups[0].now === proxyName ? 'btn-primary' : 'btn-ghost bg-base-300/50'"
						@click="selectProxy(proxyGroups[0].name, proxyName)">
						<span class="text-[10px] truncate w-full text-left font-medium">{{ proxyName
						}}</span>
						<span class="text-[9px] opacity-70" v-if="proxies[proxyName]?.history?.length"
							:style="{ color: getDelayColor(proxies[proxyName].history[proxies[proxyName].history.length - 1].delay) }">
							{{ proxies[proxyName].history[proxies[proxyName].history.length - 1].delay }}ms
						</span>
						<span class="text-[9px] opacity-30" v-else>--</span>
					</button>
				</div>
			</div>
		</div>
	</div>
</template>
