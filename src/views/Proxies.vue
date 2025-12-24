<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useMihomo } from '../composables/useMihomo';
import { useCore } from '../composables/useCore';
import { useI18n } from '../composables/useI18n';
import { useToast } from '../composables/useToast';
import { useDialog } from '../composables/useDialog';

const {
	proxies,
	config,
	selectProxy,
	testLatency,
	fetchProxies,
	updateConfig,
	coreRunning,
} = useMihomo();
const { refreshStatus } = useCore();
const { t } = useI18n();
const { success, error, warning } = useToast();
const { confirm } = useDialog();

const groupTypes = ['Selector', 'URLTest', 'Fallback', 'LoadBalance', 'Select', 'URL-Test', 'Relay', 'Direct', 'Reject'];
const isGroup = (p: any) => {
	if (!p.type) return false;
	const type = p.type.toLowerCase();
	const isKnownGroup = groupTypes.some(t => t.toLowerCase() === type);
	// Groups usually have an 'all' list or a 'now' selection
	return isKnownGroup || (Array.isArray(p.all) && p.all.length > 0);
};

const proxyGroups = computed(() =>
	Object.values(proxies.value).filter(p => isGroup(p) && !['GLOBAL', 'DIRECT', 'REJECT'].includes(p.name))
);

const allNodes = computed(() => 
    Object.values(proxies.value).filter(p => !isGroup(p) && p.name !== 'DIRECT' && p.name !== 'REJECT')
);

const showGroupModal = ref(false);
const editingGroup = ref<any>(null);
const groupForm = ref({
	name: '',
	type: 'select',
	proxies: [] as string[],
	url: 'http://www.gstatic.com/generate_204',
	interval: 300
});

const groupTypesList = computed(() => [
	{ value: 'select', label: t('proxies.typeSelector'), desc: t('proxies.typeDescSelector') },
	{ value: 'url-test', label: t('proxies.typeUrlTest'), desc: t('proxies.typeDescUrlTest') },
	{ value: 'fallback', label: t('proxies.typeFallback'), desc: t('proxies.typeDescFallback') },
	{ value: 'load-balance', label: t('proxies.typeLoadBalance'), desc: t('proxies.typeDescLoadBalance') }
]);

const handleTestAllLatency = async () => {
	for (const group of proxyGroups.value) {
		for (const proxyName of group.all || []) {
			testLatency(proxyName);
		}
	}
	setTimeout(() => fetchProxies(), 3000);
};

const getDelayColor = (delay: number) => {
	if (delay === 0) return '#666';
	if (delay < 200) return '#34C759';
	if (delay < 500) return '#FF9F0A';
	return '#FF3B30';
};

const openAddGroupModal = () => {
	editingGroup.value = null;
	groupForm.value = {
		name: '',
		type: 'select',
		proxies: [],
		url: 'http://www.gstatic.com/generate_204',
		interval: 300
	};
	showGroupModal.value = true;
};

const openEditGroupModal = (group: any) => {
	editingGroup.value = group;
	groupForm.value = {
		name: group.name,
		type: group.type.toLowerCase(),
		proxies: [...(group.all || [])],
		url: group.url || 'http://www.gstatic.com/generate_204',
		interval: group.interval || 300
	};
	showGroupModal.value = true;
};

const toggleProxyInGroup = (proxyName: string) => {
	const index = groupForm.value.proxies.indexOf(proxyName);
	if (index > -1) {
		groupForm.value.proxies.splice(index, 1);
	} else {
		groupForm.value.proxies.push(proxyName);
	}
};

const handleSaveGroup = async () => {
	if (!groupForm.value.name.trim()) {
		warning(t('proxies.groupNameRequired'));
		return;
	}
	if (groupForm.value.proxies.length === 0) {
		warning(t('proxies.groupProxyRequired'));
		return;
	}

	try {
		const currentGroups = config.value?.['proxy-groups'] || [];
		let updatedGroups;

		const newGroup: any = {
			name: groupForm.value.name,
			type: groupForm.value.type,
			proxies: groupForm.value.proxies
		};

		// Add URL and interval for url-test and fallback
		if (groupForm.value.type === 'url-test' || groupForm.value.type === 'fallback') {
			newGroup.url = groupForm.value.url;
			newGroup.interval = groupForm.value.interval;
		}

		if (editingGroup.value) {
			// Update existing group
			updatedGroups = currentGroups.map((g: any) =>
				g.name === editingGroup.value.name ? newGroup : g
			);
		} else {
			// Add new group
			updatedGroups = [...currentGroups, newGroup];
		}

		await updateConfig({ 'proxy-groups': updatedGroups });

		showGroupModal.value = false;
		success(t('proxies.groupSaved'));

		// Refresh proxies after a short delay
		setTimeout(() => fetchProxies(), 1000);
	} catch (e) {
		error(t('proxies.addFailed', { error: String(e) }));
	}
};

const handleDeleteGroup = async (groupName: string) => {
	const ok = await confirm({
		title: t('actions.delete'),
		message: t('proxies.deleteConfirm', { name: groupName }),
		confirmText: t('actions.delete'),
		cancelText: t('actions.cancel')
	});
	if (!ok) return;

	try {
		const currentGroups = config.value?.['proxy-groups'] || [];
		const updatedGroups = currentGroups.filter((g: any) => g.name !== groupName);

		await updateConfig({ 'proxy-groups': updatedGroups });

		success(t('proxies.groupDeleted'));
		setTimeout(() => fetchProxies(), 1000);
	} catch (e) {
		error(t('proxies.deleteFailed', { error: String(e) }));
	}
};

onMounted(async () => {
	await refreshStatus();
	await fetchProxies(5, 500);
});
</script>

<template>
	<div class="space-y-6 animate-in slide-in-from-bottom-4 duration-500 pb-20">
		<div class="flex items-center justify-between px-1">
			<h1 class="text-3xl font-bold tracking-tight">{{ t('proxies.title') }}</h1>
			<div class="flex gap-2">
				<button class="btn btn-sm btn-primary" @click="openAddGroupModal">
					<i class="fa-solid fa-plus"></i> {{ t('proxies.addGroup') }}
				</button>
				<button class="btn btn-sm btn-ghost bg-base-200" @click="handleTestAllLatency">
					<i class="fa-solid fa-bolt"></i>
				</button>
				<button class="btn btn-sm btn-ghost bg-base-200" @click="() => { refreshStatus(); fetchProxies(3, 500); }">
					<i class="fa-solid fa-rotate"></i>
				</button>
			</div>
		</div>

		<!-- Proxy Groups -->
		<div v-if="proxyGroups.length > 0" class="space-y-4">
			<div v-for="group in proxyGroups" :key="group.name"
				class="collapse bg-base-200 shadow-sm border border-white/5">
				<input type="checkbox" :checked="group === proxyGroups[0]" />
				<div class="collapse-title px-4 py-3 cursor-pointer">
					<div class="flex items-center justify-between">
						<div class="flex items-center gap-2">
							<div class="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center text-primary">
								<i class="fa-solid fa-layer-group"></i>
							</div>
							<div>
								<h3 class="font-bold text-sm">{{ group.name }}</h3>
								<p class="text-[10px] opacity-50 uppercase tracking-wider">{{ group.type }}</p>
							</div>
						</div>
						<div class="flex items-center gap-2">
							<div class="badge badge-primary badge-outline text-[10px]">{{ group.now }}</div>
							<button class="btn btn-xs btn-square btn-ghost" @click.stop="openEditGroupModal(group)"
								:title="t('proxies.editGroup')">
								<i class="fa-solid fa-pen text-[10px]"></i>
							</button>
							<button class="btn btn-xs btn-square btn-ghost text-error"
								@click.stop="handleDeleteGroup(group.name)" :title="t('proxies.deleteGroup')">
								<i class="fa-solid fa-trash text-[10px]"></i>
							</button>
						</div>
					</div>
				</div>
				<div class="collapse-content px-4 pb-4">
					<div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-2">
						<button v-for="proxyName in group.all" :key="proxyName"
							class="btn btn-sm h-auto py-2 flex flex-col items-start gap-0.5"
							:class="group.now === proxyName ? 'btn-primary' : 'btn-ghost bg-base-300/50'"
							@click="selectProxy(group.name, proxyName)">
							<span class="text-[10px] truncate w-full text-left font-medium">{{ proxyName }}</span>
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

		<!-- All Nodes Section -->
		<div v-if="allNodes.length > 0" class="space-y-4">
			<h2 class="text-xs font-bold uppercase opacity-40 tracking-widest">{{ t('proxies.allNodes') }} ({{ allNodes.length }})</h2>
			<div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-3">
				<div v-for="proxy in allNodes" :key="proxy.name"
					class="p-3 rounded-xl bg-base-200 border border-white/5 flex flex-col gap-2 shadow-sm hover:border-primary/30 transition-colors">
					<div class="flex flex-col min-w-0">
						<span class="text-[11px] font-bold truncate">{{ proxy.name }}</span>
						<span class="text-[9px] opacity-40 uppercase tracking-tighter">{{ proxy.type }}</span>
					</div>
					<div class="flex items-center justify-between mt-1">
						<span class="text-[9px] font-mono" v-if="proxy.history?.length"
							:style="{ color: getDelayColor(proxy.history[proxy.history.length - 1].delay) }">
							{{ proxy.history[proxy.history.length - 1].delay }}ms
						</span>
						<span class="text-[9px] opacity-20" v-else>--</span>
						<button class="btn btn-xs btn-circle btn-ghost" @click="testLatency(proxy.name)">
							<i class="fa-solid fa-bolt text-[8px]"></i>
						</button>
					</div>
				</div>
			</div>
		</div>

		<!-- Empty State & Diagnostics -->
		<div v-if="proxyGroups.length === 0 && allNodes.length === 0" 
			class="flex flex-col items-center justify-center py-20 opacity-40">
			<i class="fa-solid fa-ghost text-6xl mb-4"></i>
			<p class="text-lg font-medium">{{ t('profiles.noProxies') }}</p>
			
			<div class="mt-8 p-6 bg-base-300/30 rounded-2xl border border-white/5 max-w-lg w-full">
				<div class="flex items-center gap-2 mb-4 text-primary font-bold text-xs uppercase tracking-widest">
					<i class="fa-solid fa-microchip"></i> {{ t('proxies.diagnosticsTitle') }}
				</div>
				<div class="space-y-2 text-[11px] font-mono">
					<div class="flex justify-between border-b border-white/5 pb-1">
						<span>{{ t('proxies.coreRunning') }}</span>
						<span :class="coreRunning ? 'text-success' : 'text-error'">{{ coreRunning }}</span>
					</div>
					<div class="flex justify-between border-b border-white/5 pb-1">
						<span>{{ t('proxies.totalFetched') }}</span>
						<span>{{ Object.keys(proxies).length }}</span>
					</div>
					<div v-if="Object.keys(proxies).length > 0" class="mt-4">
						<div class="opacity-50 mb-1">{{ t('proxies.fetchedNodes') }}</div>
						<div class="bg-black/20 p-2 rounded text-[9px] whitespace-pre-wrap">
							{{ Object.keys(proxies).slice(0, 10).join(', ') }}
						</div>
					</div>
				</div>
				<div class="mt-6 text-[10px] italic opacity-60">
					{{ t('proxies.diagnosticsTip') }}
				</div>
			</div>
		</div>

		<!-- Add/Edit Group Modal -->
		<div v-if="showGroupModal" class="fixed inset-0 z-[1100] flex items-center justify-center p-4">
			<div class="absolute inset-0 bg-black/40 backdrop-blur-sm" @click="showGroupModal = false"></div>
			<div class="modal-box relative max-w-2xl bg-base-100 border border-white/10 shadow-2xl">
				<h3 class="font-bold text-lg mb-6">{{ t('proxies.addProxyGroupTitle', {
					mode: editingGroup ? t('actions.edit') : t('actions.add')
				}) }}</h3>

				<div class="space-y-4">
					<div class="grid grid-cols-2 gap-4">
						<div class="form-control w-full">
							<label class="label">
								<span class="label-text text-xs font-medium">{{ t('proxies.groupName') }}</span>
							</label>
							<input type="text" v-model="groupForm.name" :disabled="!!editingGroup"
								:placeholder="t('proxies.proxyGroup')"
								class="input input-bordered input-sm bg-base-200 border-none focus:outline-none" />
						</div>

						<div class="form-control w-full">
							<label class="label">
								<span class="label-text text-xs font-medium">{{ t('proxies.groupType') }}</span>
							</label>
							<select v-model="groupForm.type"
								class="select select-bordered select-sm bg-base-200 border-none focus:outline-none">
								<option v-for="type in groupTypesList" :key="type.value" :value="type.value">
									{{ type.label }}
								</option>
							</select>
						</div>
					</div>

					<!-- URL and Interval for url-test and fallback -->
					<div v-if="groupForm.type === 'url-test' || groupForm.type === 'fallback'"
						class="grid grid-cols-2 gap-4">
						<div class="form-control w-full">
							<label class="label">
								<span class="label-text text-xs font-medium">{{ t('proxies.testUrl') }}</span>
							</label>
							<input type="text" v-model="groupForm.url" placeholder="http://www.gstatic.com/generate_204"
								class="input input-bordered input-sm bg-base-200 border-none focus:outline-none" />
						</div>

						<div class="form-control w-full">
							<label class="label">
								<span class="label-text text-xs font-medium">{{ t('proxies.intervalSeconds') }}</span>
							</label>
							<input type="number" v-model.number="groupForm.interval" min="60"
								class="input input-bordered input-sm bg-base-200 border-none focus:outline-none" />
						</div>
					</div>

					<div class="form-control w-full">
						<label class="label">
							<span class="label-text text-xs font-medium">{{ t('proxies.selectProxies') }}</span>
							<span class="label-text-alt text-xs">{{ t('common.selected', { count: groupForm.proxies.length }) }}</span>
						</label>
						<div class="bg-base-200 rounded-lg p-3 max-h-64 overflow-y-auto">
							<div class="grid grid-cols-2 gap-2">
								<!-- Individual Nodes -->
								<label v-for="proxy in allNodes" :key="proxy.name"
									class="label cursor-pointer justify-start gap-3 p-2 rounded hover:bg-base-300/50">
									<input type="checkbox" :checked="groupForm.proxies.includes(proxy.name)"
										@change="toggleProxyInGroup(proxy.name)"
										class="checkbox checkbox-primary checkbox-sm" />
									<span class="label-text text-xs">{{ proxy.name }}</span>
								</label>

								<!-- Other Groups -->
								<label v-for="group in proxyGroups.filter(g => g.name !== editingGroup?.name)"
									:key="group.name"
									class="label cursor-pointer justify-start gap-3 p-2 rounded hover:bg-base-300/50">
									<input type="checkbox" :checked="groupForm.proxies.includes(group.name)"
										@change="toggleProxyInGroup(group.name)"
										class="checkbox checkbox-primary checkbox-sm" />
									<span class="label-text text-xs font-bold">{{ group.name }}</span>
								</label>
							</div>
						</div>
					</div>
				</div>

				<div class="modal-action">
					<button class="btn btn-ghost" @click="showGroupModal = false">{{ t('actions.cancel') }}</button>
					<button class="btn btn-primary px-8" @click="handleSaveGroup">
						{{ editingGroup ? t('actions.save') : t('actions.add') }} {{ t('proxies.group') }}
					</button>
				</div>
			</div>
		</div>
	</div>
</template>
