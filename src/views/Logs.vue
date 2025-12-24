<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue';
import { useMihomo } from '../composables/useMihomo';
import { useI18n } from '../composables/useI18n';

const { logs } = useMihomo();
const logFilter = ref('all');
const searchQuery = ref('');
const { t } = useI18n();

// Virtual scroll state
const containerRef = ref<HTMLElement | null>(null);
const scrollTop = ref(0);
const containerHeight = ref(400);
const itemHeight = 28; // Height per log row
const overscan = 10;

// Auto-scroll & pause
const autoScroll = ref(true);
const isPaused = ref(false);
const userScrolling = ref(false);

// Filtered logs
const filteredLogs = computed(() => {
	let result = logs.value;
	
	// Filter by level
	if (logFilter.value !== 'all') {
		result = result.filter(l => l.type.toLowerCase() === logFilter.value);
	}
	
	// Filter by search query
	if (searchQuery.value.trim()) {
		const query = searchQuery.value.toLowerCase();
		result = result.filter(l => l.payload.toLowerCase().includes(query));
	}
	
	return result;
});

// Virtual scroll calculations
const totalHeight = computed(() => filteredLogs.value.length * itemHeight);

const visibleRange = computed(() => {
	const start = Math.max(0, Math.floor(scrollTop.value / itemHeight) - overscan);
	const visibleCount = Math.ceil(containerHeight.value / itemHeight) + overscan * 2;
	const end = Math.min(filteredLogs.value.length, start + visibleCount);
	return { start, end };
});

const visibleItems = computed(() => {
	const { start, end } = visibleRange.value;
	return filteredLogs.value.slice(start, end).map((log, index) => ({
		...log,
		virtualIndex: start + index
	}));
});

const offsetY = computed(() => visibleRange.value.start * itemHeight);

const clearLogs = () => logs.value = [];

const togglePause = () => {
	isPaused.value = !isPaused.value;
	if (!isPaused.value) {
		autoScroll.value = true;
	}
};

const levels = computed(() => [
	{ value: 'all', label: t('logs.levelAll') },
	{ value: 'debug', label: t('logs.levelDebug') },
	{ value: 'info', label: t('logs.levelInfo') },
	{ value: 'warning', label: t('logs.levelWarning') },
	{ value: 'error', label: t('logs.levelError') }
]);

// Scroll handling
const handleScroll = (e: Event) => {
	const target = e.target as HTMLElement;
	scrollTop.value = target.scrollTop;

	const atBottom = target.scrollHeight - target.scrollTop - target.clientHeight < 30;
	if (!userScrolling.value) {
		autoScroll.value = atBottom;
	}
};

const handleWheel = () => {
	userScrolling.value = true;
	setTimeout(() => {
		userScrolling.value = false;
	}, 150);
};

// Scroll to bottom when new logs arrive (if auto-scroll enabled and not paused)
watch(() => logs.value.length, () => {
	if (autoScroll.value && !isPaused.value && containerRef.value) {
		nextTick(() => {
			if (containerRef.value) {
				containerRef.value.scrollTop = containerRef.value.scrollHeight;
			}
		});
	}
});

// Reset scroll position when filter changes
watch([logFilter, searchQuery], () => {
	scrollTop.value = 0;
	if (containerRef.value) {
		containerRef.value.scrollTop = 0;
	}
});

// Resize observer
let resizeObserver: ResizeObserver | null = null;

onMounted(() => {
	if (containerRef.value) {
		containerHeight.value = containerRef.value.clientHeight;
		resizeObserver = new ResizeObserver((entries) => {
			for (const entry of entries) {
				containerHeight.value = entry.contentRect.height;
			}
		});
		resizeObserver.observe(containerRef.value);
	}
});

onUnmounted(() => {
	if (resizeObserver) {
		resizeObserver.disconnect();
	}
});

// Get log level badge class
const getLevelClass = (type: string) => {
	switch (type.toLowerCase()) {
		case 'debug': return 'text-base-content/50';
		case 'info': return 'text-info';
		case 'warning': return 'text-warning';
		case 'error': return 'text-error';
		default: return 'text-base-content/70';
	}
};
</script>

<template>
	<div class="h-full flex flex-col animate-in slide-in-from-bottom-4 duration-500">
		<!-- Header Row: Title + Actions -->
		<div class="flex items-center justify-between mb-4 flex-shrink-0">
			<h1 class="text-2xl font-bold">{{ t('logs.title') }}</h1>
			<div class="flex gap-2 items-center">
				<!-- Pause Button -->
				<button 
					class="btn btn-sm btn-circle" 
					:class="isPaused ? 'btn-warning' : 'btn-ghost'"
					@click="togglePause"
					:title="isPaused ? t('logs.resume') : t('logs.pause')">
					<i :class="isPaused ? 'fa-solid fa-play' : 'fa-solid fa-pause'"></i>
				</button>
				<!-- Clear Button -->
				<button class="btn btn-sm btn-error" @click="clearLogs">
					{{ t('logs.clear') }}
				</button>
			</div>
		</div>

		<!-- Filter Row: Level Dropdown + Search -->
		<div class="flex items-center gap-3 mb-4 flex-shrink-0">
			<!-- Level Filter Dropdown -->
			<select 
				v-model="logFilter" 
				class="select select-sm select-bordered w-28 bg-base-200">
				<option v-for="level in levels" :key="level.value" :value="level.value">
					{{ level.label }}
				</option>
			</select>
			
			<!-- Search Input -->
			<div class="flex-1 relative">
				<input 
					type="text" 
					v-model="searchQuery"
					:placeholder="t('logs.filterPlaceholder')"
					class="input input-sm input-bordered w-full bg-base-200 pl-9" />
				<i class="fa-solid fa-filter absolute left-3 top-1/2 -translate-y-1/2 text-base-content/30 text-xs"></i>
			</div>
			
			<!-- Log Count -->
			<div class="badge badge-ghost text-xs">
				{{ filteredLogs.length }}
			</div>
		</div>

		<!-- Log Content Area -->
		<div 
			ref="containerRef"
			class="flex-1 bg-neutral text-neutral-content rounded-xl font-mono text-xs overflow-y-auto shadow-inner border border-white/5"
			@scroll="handleScroll" 
			@wheel="handleWheel">
			
			<!-- Virtual scroll spacer -->
			<div :style="{ height: totalHeight + 'px', position: 'relative' }">
				<div :style="{ transform: `translateY(${offsetY}px)` }">
					<div 
						v-for="log in visibleItems" 
						:key="log.virtualIndex" 
						class="flex items-center gap-3 px-4 hover:bg-white/5 transition-colors"
						:style="{ height: itemHeight + 'px' }">
						<!-- Time -->
						<span class="text-[10px] opacity-40 flex-shrink-0 w-14 font-normal">
							{{ log.time }}
						</span>
						<!-- Level Badge -->
						<span 
							class="text-[10px] font-bold uppercase w-16 flex-shrink-0"
							:class="getLevelClass(log.type)">
							{{ log.type }}
						</span>
						<!-- Message -->
						<span 
							class="truncate flex-1 opacity-90" 
							:title="log.payload">
							{{ log.payload }}
						</span>
					</div>
				</div>
			</div>
			
			<!-- Empty state -->
			<div 
				v-if="filteredLogs.length === 0"
				class="absolute inset-0 flex flex-col items-center justify-center opacity-20 pointer-events-none">
				<i class="fa-solid fa-inbox text-4xl mb-2"></i>
				<span>{{ t('logs.empty') }}</span>
			</div>
		</div>
	</div>
</template>
