<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { useMihomo } from '../composables/useMihomo';
import { useI18n } from '../composables/useI18n';

const { connections, closeAllConnections } = useMihomo();
const { t } = useI18n();
const searchConnections = ref('');

const handleCloseAllConnections = async () => await closeAllConnections();

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

const getHostKey = (c: typeof connections.value[number]) =>
    c.metadata.host || c.metadata.destinationIP || 'unknown';

const hostSpeeds = ref<Record<string, { down: number; up: number }>>({});
const previousTotals = ref<Record<string, { down: number; up: number; at: number }>>({});

watch(() => connections.value, (next) => {
    const totals: Record<string, { down: number; up: number }> = {};
    for (const c of next) {
        const host = getHostKey(c);
        if (!totals[host]) {
            totals[host] = { down: 0, up: 0 };
        }
        totals[host].down += c.download || 0;
        totals[host].up += c.upload || 0;
    }

    const now = Date.now();
    const nextSpeeds: Record<string, { down: number; up: number }> = {};
    const nextPrevious: Record<string, { down: number; up: number; at: number }> = {};
    Object.entries(totals).forEach(([host, total]) => {
        const prev = previousTotals.value[host];
        let down = 0;
        let up = 0;
        if (prev) {
            const deltaSeconds = (now - prev.at) / 1000;
            if (deltaSeconds > 0) {
                down = Math.max(0, total.down - prev.down) / deltaSeconds;
                up = Math.max(0, total.up - prev.up) / deltaSeconds;
            }
        }
        nextSpeeds[host] = { down, up };
        nextPrevious[host] = { down: total.down, up: total.up, at: now };
    });
    hostSpeeds.value = nextSpeeds;
    previousTotals.value = nextPrevious;
}, { deep: true });

const groupedConnections = computed(() => {
    const buckets: Record<string, { items: typeof connections.value; totalDown: number; totalUp: number }> = {};
    connections.value.forEach((c) => {
        const host = getHostKey(c);
        if (!buckets[host]) {
            buckets[host] = { items: [], totalDown: 0, totalUp: 0 };
        }
        buckets[host].items.push(c);
        buckets[host].totalDown += c.download || 0;
        buckets[host].totalUp += c.upload || 0;
    });

    return Object.entries(buckets).map(([host, bucket]) => {
        const sample = bucket.items[0];
        return {
            host,
            destinationIP: sample?.metadata.destinationIP || '',
            destinationPort: sample?.metadata.destinationPort || '',
            network: sample?.metadata.network || '',
            rule: sample?.rule || '',
            proxy: sample?.chains?.[0] || '',
            totalDown: bucket.totalDown,
            totalUp: bucket.totalUp,
            realtimeDown: hostSpeeds.value[host]?.down || 0,
            realtimeUp: hostSpeeds.value[host]?.up || 0,
        };
    });
});

const filteredConnections = computed(() => {
    if (!searchConnections.value) return groupedConnections.value;
    const query = searchConnections.value.toLowerCase();
    return groupedConnections.value.filter(c =>
        (c.host || '').toLowerCase().includes(query) ||
        (c.destinationIP || '').toLowerCase().includes(query) ||
        (c.rule || '').toLowerCase().includes(query) ||
        (c.proxy || '').toLowerCase().includes(query)
    );
});

const sortKey = ref<'totalDown' | 'totalUp' | 'realtimeDown' | 'realtimeUp' | ''>('');
const sortDirection = ref<'asc' | 'desc'>('desc');
const setSort = (key: typeof sortKey.value) => {
    if (sortKey.value === key) {
        sortDirection.value = sortDirection.value === 'asc' ? 'desc' : 'asc';
        return;
    }
    sortKey.value = key;
    sortDirection.value = 'desc';
};

const sortedConnections = computed(() => {
    const key = sortKey.value;
    if (!key) return filteredConnections.value;
    const dir = sortDirection.value === 'asc' ? 1 : -1;
    return [...filteredConnections.value].sort((a, b) => {
        const left = a[key];
        const right = b[key];
        if (left === right) return 0;
        return left > right ? dir : -dir;
    });
});
</script>

<template>
    <div class="space-y-6 animate-in slide-in-from-bottom-4 duration-500">
        <div class="flex items-center justify-between">
            <h1 class="text-3xl font-bold tracking-tight">{{ t('connections.title') }}</h1>
            <div class="flex items-center gap-3">
                <div class="badge badge-ghost">{{ t('connections.active', { count: connections.length }) }}</div>
                <button class="btn btn-sm btn-error btn-outline" @click="handleCloseAllConnections">
                    <i class="fa-solid fa-xmark"></i> {{ t('actions.closeAll') }}
                </button>
            </div>
        </div>

        <div class="join w-full shadow-sm">
            <div class="join-item bg-base-200 flex items-center px-4">
                <i class="fa-solid fa-search opacity-50"></i>
            </div>
            <input type="text" v-model="searchConnections" :placeholder="t('connections.searchPlaceholder')"
                class="input input-bordered join-item w-full bg-base-200 border-none focus:outline-none" />
        </div>

        <div class="overflow-x-auto rounded-xl border border-white/5 bg-base-200 shadow-sm">
            <table class="table table-zebra table-sm">
                <thead>
                    <tr class="bg-base-300/50">
                        <th class="py-3">{{ t('connections.tableHost') }}</th>
                        <th>{{ t('connections.tableNetwork') }}</th>
                        <th>{{ t('connections.tableRule') }}</th>
                        <th>{{ t('connections.tableProxy') }}</th>
                        <th class="cursor-pointer select-none" @click="setSort('totalDown')">
                            <span class="inline-flex items-center gap-1">
                                {{ t('connections.tableTotalDownload') }}
                                <i v-if="sortKey === 'totalDown'" class="fa-solid" :class="sortDirection === 'asc' ? 'fa-arrow-up' : 'fa-arrow-down'"></i>
                            </span>
                        </th>
                        <th class="cursor-pointer select-none" @click="setSort('totalUp')">
                            <span class="inline-flex items-center gap-1">
                                {{ t('connections.tableTotalUpload') }}
                                <i v-if="sortKey === 'totalUp'" class="fa-solid" :class="sortDirection === 'asc' ? 'fa-arrow-up' : 'fa-arrow-down'"></i>
                            </span>
                        </th>
                        <th class="cursor-pointer select-none" @click="setSort('realtimeDown')">
                            <span class="inline-flex items-center gap-1">
                                {{ t('connections.tableDownload') }}
                                <i v-if="sortKey === 'realtimeDown'" class="fa-solid" :class="sortDirection === 'asc' ? 'fa-arrow-up' : 'fa-arrow-down'"></i>
                            </span>
                        </th>
                        <th class="cursor-pointer select-none" @click="setSort('realtimeUp')">
                            <span class="inline-flex items-center gap-1">
                                {{ t('connections.tableUpload') }}
                                <i v-if="sortKey === 'realtimeUp'" class="fa-solid" :class="sortDirection === 'asc' ? 'fa-arrow-up' : 'fa-arrow-down'"></i>
                            </span>
                        </th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="c in sortedConnections" :key="c.host" class="hover">
                        <td class="max-w-xs">
                            <div class="flex flex-col">
                                <span class="font-medium truncate text-[11px]">{{ c.host }}</span>
                                <span class="text-[9px] opacity-50">{{ c.destinationIP }}:{{ c.destinationPort }}</span>
                            </div>
                        </td>
                        <td>
                            <div class="badge badge-ghost badge-xs">{{ c.network?.toUpperCase?.() || '--' }}</div>
                        </td>
                        <td><span class="text-[10px] opacity-70">{{ c.rule || '--' }}</span></td>
                        <td><span class="text-[10px] font-bold text-primary">{{ c.proxy || '--' }}</span></td>
                        <td class="font-mono text-[11px]">{{ formatBytes(c.totalDown) }}</td>
                        <td class="font-mono text-[11px]">{{ formatBytes(c.totalUp) }}</td>
                        <td class="font-mono text-[11px]">{{ formatSpeed(c.realtimeDown) }}</td>
                        <td class="font-mono text-[11px]">{{ formatSpeed(c.realtimeUp) }}</td>
                    </tr>
                </tbody>
            </table>
        </div>
    </div>
</template>
