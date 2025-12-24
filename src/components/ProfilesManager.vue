<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { Codemirror } from 'vue-codemirror';
import { yaml } from '@codemirror/lang-yaml';
import { oneDark } from '@codemirror/theme-one-dark';
import { useProfiles } from '../composables/useProfiles';
import { parseConfig, parseProxyUrl } from '../api/profiles';
import { useI18n } from '../composables/useI18n';
import { useToast } from '../composables/useToast';
import { useDialog } from '../composables/useDialog';
import { open } from '@tauri-apps/plugin-dialog';
import { getPrivilegedHelperStatus } from '../api/tauri';
import { isMac } from '../utils/platform';

const emit = defineEmits<{
	(e: 'apply'): void;
	(e: 'back'): void;
	(e: 'refresh'): void;
}>();

const {
	profiles,
	activeProfile,
	currentContent,
	error: profileError,
	fetchProfiles: _fetchProfiles,
	createProfile,
	deleteProfile,
	setActive,
	loadContent,
	saveContent,
	renameProfile: _renameProfile,
	updateFromUrl,
	addProxy,
	saveConfigObj,
} = useProfiles();
const { t } = useI18n();
const { success, error, warning } = useToast();
const { confirm, prompt } = useDialog();
const isMacOS = isMac();

// Custom debounce function
function debounce<T extends (...args: any[]) => void>(fn: T, delay: number): T {
	let timeoutId: ReturnType<typeof setTimeout> | null = null;
	return ((...args: Parameters<T>) => {
		if (timeoutId) clearTimeout(timeoutId);
		timeoutId = setTimeout(() => fn(...args), delay);
	}) as T;
}

const selectedProfileId = ref<string | null>(null);
const editMode = ref<'visual' | 'yaml'>('visual');
const showNewProfileModal = ref(false);
const newProfileName = ref('');
const newProfileUrl = ref('');
const newProfilePath = ref('');
const newProfileSource = ref<'empty' | 'url' | 'file'>('empty');
const yamlExtensions = [yaml(), oneDark];

// Parsed config for visual editor
const parsedConfig = ref<any>({});

// Visual editor state
const editProxy = ref<any>(null);
const editProxyIndex = ref(-1);
const showProxyModal = ref(false);

const editGroup = ref<any>(null);
const editGroupIndex = ref(-1);
const showGroupModal = ref(false);

const proxyTypes = ['ss', 'vmess', 'vless', 'trojan', 'hysteria', 'hysteria2', 'tuic', 'wireguard', 'socks5', 'http'];
const groupTypes = ['select', 'url-test', 'fallback', 'load-balance'];

watch(selectedProfileId, async (id) => {
	if (id) {
		await loadContent(id);
		await parseCurrentConfig();
	}
});

// Set initial selection to active profile on mount
// When entering the profiles page, always show the active profile
watch([profiles, activeProfile], () => {
	if (profiles.value.length > 0) {
		// If there's an active profile, always select it when entering the page
		// This ensures the active profile is shown when user navigates to this page
		if (activeProfile.value?.id) {
			selectedProfileId.value = activeProfile.value.id;
		} else if (!selectedProfileId.value) {
			// No active profile, fallback to first profile only if nothing selected
			selectedProfileId.value = profiles.value[0].id;
		}
	}
}, { immediate: true });

// Shared save function
const saveCurrentProfile = async () => {
	if (!selectedProfileId.value) return;
	try {
		if (editMode.value === 'visual') {
			await saveConfigObj(selectedProfileId.value, parsedConfig.value);
		} else {
			await saveContent(selectedProfileId.value, currentContent.value);
		}
	} catch (e) {
		error(t('profiles.saveFailed', { error: String(e) }));
		throw e;
	}
};

// Auto-save debounced function
const debouncedSave = debounce(async () => {
	try {
		await saveCurrentProfile();
	} catch (e) {
		// Error handled in saveCurrentProfile
	}
}, 1000);

// Watch parsedConfig for auto-save (visual mode)
watch(parsedConfig, () => {
	if (editMode.value === 'visual' && selectedProfileId.value) {
		debouncedSave();
	}
}, { deep: true });

// Watch currentContent for auto-save (yaml mode)
watch(currentContent, () => {
	if (editMode.value === 'yaml' && selectedProfileId.value) {
		debouncedSave();
	}
});

const parseCurrentConfig = async () => {
	if (!currentContent.value) return;
	try {
		parsedConfig.value = await parseConfig(currentContent.value);
	} catch (e) {
		console.error('Failed to parse config:', e);
		// Fallback to empty config if parsing fails
		parsedConfig.value = {
			proxies: [],
			'proxy-groups': [],
			rules: []
		};
	}
};

const selectedProfile = computed(() =>
	profiles.value.find(p => p.id === selectedProfileId.value)
);

const handleCreateProfile = async () => {
	if (!newProfileName.value.trim()) return;

	try {
		if (newProfileSource.value === 'url' && !newProfileUrl.value.trim()) {
			warning(t('profiles.createMissingUrl'));
			return;
		}
		if (newProfileSource.value === 'file' && !newProfilePath.value.trim()) {
			warning(t('profiles.createMissingPath'));
			return;
		}

		const sourceUrl = newProfileSource.value === 'url'
			? newProfileUrl.value.trim()
			: undefined;
		const sourcePath = newProfileSource.value === 'file'
			? newProfilePath.value.trim()
			: undefined;

		const profile = await createProfile(
			newProfileName.value.trim(),
			sourceUrl,
			sourcePath
		);
		selectedProfileId.value = profile.id;

		// If URL provided, download immediately
		if (newProfileSource.value === 'url' && newProfileUrl.value.trim()) {
			try {
				await updateFromUrl(profile.id);
			} catch (e) {
				console.error('Failed to initial download:', e);
			}
		}

		showNewProfileModal.value = false;
		newProfileName.value = '';
		newProfileUrl.value = '';
		newProfilePath.value = '';
		newProfileSource.value = 'empty';
	} catch (e) {
		error(t('profiles.createFailed', { error: String(e) }));
	}
};

const handlePickProfileFile = async () => {
	try {
		const selected = await open({
			multiple: false,
			filters: [{ name: 'YAML', extensions: ['yml', 'yaml'] }],
		});
		if (typeof selected === 'string') {
			newProfilePath.value = selected;
		}
	} catch (e) {
		error(t('profiles.filePickFailed', { error: String(e) }));
	}
};

const handleSwitchSource = (next: 'empty' | 'url' | 'file') => {
	newProfileSource.value = next;
	if (next !== 'url') newProfileUrl.value = '';
	if (next !== 'file') newProfilePath.value = '';
};

const handleDeleteProfile = async (id: string) => {
	const ok = await confirm({
		title: t('actions.delete'),
		message: t('profiles.deleteConfirm'),
		confirmText: t('actions.delete'),
		cancelText: t('actions.cancel')
	});
	if (!ok) return;

	try {
		await deleteProfile(id);
		if (selectedProfileId.value === id) {
			selectedProfileId.value = activeProfile.value?.id || profiles.value[0]?.id || null;
		}
	} catch (e) {
		error(t('profiles.deleteFailed', { error: String(e) }));
	}
};

const handleSetActive = async (id: string) => {
	try {
		// 1. If activating the currently edited profile, save it first
		if (selectedProfileId.value === id) {
			await saveCurrentProfile();
		}

		// 2. Check for TUN requirement
		let configToCheck = parsedConfig.value;
		
		if (selectedProfileId.value !== id) {
			// Fetch content if not current
			await loadContent(id);
			configToCheck = await parseConfig(currentContent.value);
		} else if (editMode.value === 'yaml') {
			// If current and in yaml mode, re-parse to be sure
			await parseCurrentConfig();
			configToCheck = parsedConfig.value;
		}

		const tunEnabled = configToCheck?.tun?.enable === true;
		if (isMacOS && tunEnabled) {
			const helperValid = await getPrivilegedHelperStatus();
			if (!helperValid) {
				error(t('profiles.tunRequiresHelper'));
				return;
			}
		}
		
		const ok = await confirm({
			title: t('actions.apply'),
			message: t('profiles.activateConfirm'),
			confirmText: t('actions.apply'),
			cancelText: t('actions.cancel')
		});
		if (!ok) return;

		await setActive(id);
		selectedProfileId.value = id;
		emit('apply'); // This will trigger a core restart in the parent component
	} catch (e) {
		error(t('profiles.activateFailed', { error: String(e) }));
	}
};


const handleUpdateFromUrl = async () => {
	if (!selectedProfileId.value) return;
	if (!selectedProfile.value?.url) {
		warning(t('profiles.missingUrl'));
		return;
	}

	try {
		await updateFromUrl(selectedProfileId.value);
		await loadContent(selectedProfileId.value);
		await parseCurrentConfig();
		success(t('profiles.updateSuccess'));
	} catch (e) {
		error(t('profiles.updateFailed', { error: String(e) }));
	}
};


const normalizeProxyType = (value: string | undefined) => (value || '').toLowerCase();

const isVmessOrVless = computed(() => ['vmess', 'vless'].includes(normalizeProxyType(editProxy.value?.type)));
const isVmess = computed(() => normalizeProxyType(editProxy.value?.type) === 'vmess');

const openAddProxy = () => {
	editProxy.value = {
		name: '',
		type: 'ss',
		server: '',
		port: 443,
		password: '',
		cipher: 'aes-256-gcm',
		udp: true,
	};
	editProxyIndex.value = -1;
	showProxyModal.value = true;
};

const openEditProxy = (index: number) => {
	const proxy = { ...parsedConfig.value.proxies[index] };
	proxy.type = normalizeProxyType(proxy.type);
	editProxy.value = proxy;
	editProxyIndex.value = index;
	showProxyModal.value = true;
};

const saveProxy = () => {
	if (!editProxy.value.name || !editProxy.value.server) {
		warning(t('profiles.nameRequired'));
		return;
	}

	if (editProxyIndex.value === -1) {
		parsedConfig.value.proxies.push({ ...editProxy.value });
	} else {
		parsedConfig.value.proxies[editProxyIndex.value] = { ...editProxy.value };
	}
	showProxyModal.value = false;
};

const deleteProxy = (index: number) => {
	parsedConfig.value.proxies.splice(index, 1);
};

const openAddGroup = () => {
	editGroup.value = {
		name: '',
		type: 'select',
		proxies: [],
		url: 'http://www.gstatic.com/generate_204',
		interval: 300
	};
	editGroupIndex.value = -1;
	showGroupModal.value = true;
};

const openEditGroup = (index: number) => {
	const group = { ...parsedConfig.value['proxy-groups'][index] };
	editGroup.value = group;
	editGroupIndex.value = index;
	showGroupModal.value = true;
};

const saveGroup = () => {
	if (!editGroup.value.name) {
		warning(t('profiles.nameRequired'));
		return;
	}
	if (!parsedConfig.value['proxy-groups']) {
		parsedConfig.value['proxy-groups'] = [];
	}

	if (editGroupIndex.value === -1) {
		parsedConfig.value['proxy-groups'].push({ ...editGroup.value });
	} else {
		parsedConfig.value['proxy-groups'][editGroupIndex.value] = { ...editGroup.value };
	}
	showGroupModal.value = false;
};

const deleteGroup = (index: number) => {
	parsedConfig.value['proxy-groups'].splice(index, 1);
};

const toggleProxyInGroup = (proxyName: string) => {
	const index = editGroup.value.proxies.indexOf(proxyName);
	if (index > -1) {
		editGroup.value.proxies.splice(index, 1);
	} else {
		editGroup.value.proxies.push(proxyName);
	}
};

const addRule = async () => {
	const rule = await prompt({
		title: t('profiles.addRule'),
		message: t('profiles.addRulePrompt'),
		confirmText: t('actions.add'),
		cancelText: t('actions.cancel'),
		inputPlaceholder: t('profiles.addRulePrompt')
	});
	if (rule && rule.trim()) {
		parsedConfig.value.rules = parsedConfig.value.rules || [];
		parsedConfig.value.rules.unshift(rule.trim());
	}
};

const deleteRule = (index: number) => {
	parsedConfig.value.rules.splice(index, 1);
};

const handleImportProxyUrl = async () => {
	const url = await prompt({
		title: t('profiles.importUrl'),
		message: t('profiles.importProxyUrlPrompt'),
		confirmText: t('actions.add'),
		cancelText: t('actions.cancel'),
		inputPlaceholder: t('profiles.importProxyUrlPrompt')
	});
	if (!url) return;

	try {
		const proxy = await parseProxyUrl(url);
		if (selectedProfileId.value) {
			await addProxy(selectedProfileId.value, proxy);
		success(t('profiles.proxyAdded'));
		} else {
			// If no profile selected, just add to memory of current parsed config
			parsedConfig.value.proxies = parsedConfig.value.proxies || [];
			parsedConfig.value.proxies.push(proxy);
		}
	} catch (e) {
		error(t('profiles.importFailed', { error: String(e) }));
	}
};
</script>

<template>
	<div
		class="w-full h-full bg-base-100 rounded-2xl shadow-2xl flex flex-col overflow-hidden border border-white/5 animate-in fade-in duration-300">
		<!-- Header -->
		<div class="px-6 py-4 border-b border-white/5 flex items-center justify-between bg-base-200/50">
			<div class="flex items-center gap-3">
				<button class="btn btn-sm btn-ghost btn-square lg:hidden" @click="emit('back')">
					<i class="fa-solid fa-arrow-left"></i>
				</button>
				<div class="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center text-primary">
					<i class="fa-solid fa-folder-tree"></i>
				</div>
				<h2 class="text-lg font-bold">{{ t('profiles.title') }}</h2>
			</div>
			<div v-if="profileError" class="badge badge-error gap-2 py-3">
				<i class="fa-solid fa-circle-exclamation"></i>
				{{ profileError }}
			</div>
		</div>

		<div class="flex-1 flex overflow-hidden">
			<!-- Sidebar: Profile List -->
			<div class="w-72 border-r border-white/5 flex flex-col bg-base-200/30">
				<div class="p-4 flex items-center justify-between">
					<span class="text-[10px] uppercase font-bold tracking-widest opacity-40">{{ t('profiles.profiles') }}</span>
					<button class="btn btn-xs btn-primary btn-square" @click="showNewProfileModal = true">
						<i class="fa-solid fa-plus"></i>
					</button>
				</div>

				<div class="flex-1 overflow-y-auto px-2 pb-4 space-y-1">
					<div v-for="profile in profiles" :key="profile.id"
						class="group relative flex flex-col p-3 rounded-xl cursor-pointer transition-all duration-200"
						:class="selectedProfileId === profile.id ? 'bg-primary text-white shadow-lg shadow-primary/20' : 'hover:bg-base-300/50'"
						@click="selectedProfileId = profile.id">

						<div class="flex items-center gap-2 mb-1">
							<span class="font-bold text-sm truncate">{{ profile.name }}</span>
							<div v-if="profile.is_active"
								class="badge badge-success badge-xs text-[8px] font-black uppercase shrink-0">{{ t('profiles.active') }}</div>
						</div>

						<span class="text-[10px] opacity-50">{{ profile.updated_at }}</span>

						<!-- Hover Actions -->
						<div
							class="absolute right-2 top-1/2 -translate-y-1/2 flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
							<button v-if="!profile.is_active"
								class="btn btn-xs btn-square btn-ghost bg-base-100/20 text-white"
								@click.stop="handleSetActive(profile.id)" :title="t('profiles.activate')">
								<i class="fa-solid fa-check"></i>
							</button>
							<button v-if="profile.url" class="btn btn-xs btn-square btn-ghost bg-base-100/20 text-white"
								@click.stop="handleUpdateFromUrl" :title="t('profiles.update')">
								<i class="fa-solid fa-rotate"></i>
							</button>
							<button
								class="btn btn-xs btn-square btn-ghost bg-error/20 text-error group-hover:text-white"
								@click.stop="handleDeleteProfile(profile.id)" :title="t('profiles.delete')">
								<i class="fa-solid fa-trash"></i>
							</button>
						</div>
					</div>

					<div v-if="profiles.length === 0"
						class="flex flex-col items-center justify-center py-12 opacity-20">
						<i class="fa-solid fa-inbox text-3xl mb-2"></i>
						<span class="text-xs">{{ t('profiles.noProfiles') }}</span>
					</div>
				</div>
			</div>

			<!-- Main Editor Area -->
			<div class="flex-1 flex flex-col overflow-hidden bg-base-100">
				<div v-if="selectedProfile" class="flex flex-col h-full">
					<!-- Editor Header -->
					<div class="px-6 py-3 border-b border-white/5 flex items-center justify-between bg-base-200/20">
						<div class="tabs tabs-boxed bg-base-300/50 p-1">
							<a class="tab tab-sm gap-2" :class="{ 'tab-active': editMode === 'visual' }"
								@click="editMode = 'visual'">
								<i class="fa-solid fa-sliders text-[10px]"></i> {{ t('profiles.editorVisual') }}
							</a>
							<a class="tab tab-sm gap-2" :class="{ 'tab-active': editMode === 'yaml' }"
								@click="editMode = 'yaml'">
								<i class="fa-solid fa-code text-[10px]"></i> {{ t('profiles.editorYaml') }}
							</a>
						</div>
					</div>

					<!-- Editor Content -->
				<div class="flex-1 flex flex-col" :class="editMode === 'visual' ? 'overflow-y-auto' : 'overflow-hidden'">
					<!-- Visual Editor -->
					<div v-if="editMode === 'visual'"
						class="p-6 space-y-8 animate-in slide-in-from-bottom-2 duration-300">
							<!-- Proxies -->
							<section>
								<div class="flex items-center justify-between mb-4">
									<h3 class="text-[10px] uppercase font-bold tracking-widest opacity-40">{{ t('profiles.proxies') }}
										({{ parsedConfig.proxies?.length || 0 }})</h3>
									<div class="flex gap-2">
										<button class="btn btn-xs btn-ghost bg-base-200" @click="handleImportProxyUrl">
											<i class="fa-solid fa-link"></i> {{ t('profiles.importUrl') }}
										</button>
										<button class="btn btn-xs btn-primary" @click="openAddProxy">
											<i class="fa-solid fa-plus"></i> {{ t('profiles.addProxy') }}
										</button>
									</div>
								</div>

								<div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
									<div v-for="(proxy, index) in parsedConfig.proxies" :key="index"
										class="card bg-base-200 border border-white/5 hover:border-primary/30 transition-colors group">
										<div class="card-body p-4 flex-row items-center justify-between gap-3">
											<div class="flex flex-col min-w-0 flex-1">
												<span class="font-bold text-sm truncate">{{ proxy.name }}</span>
												<div class="flex items-center gap-2 mt-1.5 flex-wrap">
													<span class="badge badge-primary badge-sm text-[10px] uppercase shrink-0">{{
														proxy.type }}</span>
													<span class="text-xs opacity-50">{{ proxy.server
													}}:{{ proxy.port }}</span>
												</div>
											</div>
											<div
												class="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
												<button class="btn btn-xs btn-square btn-ghost"
													@click="openEditProxy(Number(index))">
													<i class="fa-solid fa-pen text-[10px]"></i>
												</button>
												<button class="btn btn-xs btn-square btn-ghost text-error"
													@click="deleteProxy(Number(index))">
													<i class="fa-solid fa-trash text-[10px]"></i>
												</button>
											</div>
										</div>
									</div>

									<div v-if="!parsedConfig.proxies?.length"
										class="col-span-full py-12 flex flex-col items-center justify-center border-2 border-dashed border-white/5 rounded-2xl opacity-20">
										<i class="fa-solid fa-server text-3xl mb-2"></i>
										<span class="text-xs">{{ t('profiles.noProxies') }}</span>
									</div>
								</div>
							</section>

							<!-- Proxy Groups -->
							<section>
								<div class="flex items-center justify-between mb-4">
									<h3 class="text-[10px] uppercase font-bold tracking-widest opacity-40">{{ t('proxies.groups') }}
										({{ parsedConfig['proxy-groups']?.length || 0 }})</h3>
									<button class="btn btn-xs btn-primary" @click="openAddGroup">
										<i class="fa-solid fa-plus"></i> {{ t('proxies.addGroup') }}
									</button>
								</div>

								<div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
									<div v-for="(group, index) in parsedConfig['proxy-groups']" :key="index"
										class="card bg-base-200 border border-white/5 hover:border-primary/30 transition-colors group">
										<div class="card-body p-4 flex-row items-center justify-between gap-3">
											<div class="flex flex-col min-w-0 flex-1">
												<span class="font-bold text-sm truncate">{{ group.name }}</span>
												<div class="flex items-center gap-2 mt-1">
												<div class="badge badge-sm badge-secondary">{{ group.type }}</div>
												<span class="text-[10px] opacity-40">{{ group.proxies?.length || 0 }} {{ t('profiles.proxies') }}</span>
											</div>
										</div>
											<div
												class="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
												<button class="btn btn-xs btn-square btn-ghost"
													@click="openEditGroup(Number(index))">
													<i class="fa-solid fa-pen text-[10px]"></i>
												</button>
												<button class="btn btn-xs btn-square btn-ghost text-error"
													@click="deleteGroup(Number(index))">
													<i class="fa-solid fa-trash text-[10px]"></i>
												</button>
											</div>
										</div>
									</div>

									<div v-if="!parsedConfig['proxy-groups']?.length"
										class="col-span-full py-12 flex flex-col items-center justify-center border-2 border-dashed border-white/5 rounded-2xl opacity-20">
										<i class="fa-solid fa-layer-group text-3xl mb-2"></i>
										<span class="text-xs">{{ t('proxies.noGroups') }}</span>
									</div>
								</div>
							</section>

							<!-- Rules -->
							<section>
								<div class="flex items-center justify-between mb-4">
									<h3 class="text-[10px] uppercase font-bold tracking-widest opacity-40">{{ t('profiles.rules') }} ({{
										parsedConfig.rules?.length || 0 }})</h3>
									<button class="btn btn-xs btn-primary" @click="addRule">
										<i class="fa-solid fa-plus"></i> {{ t('profiles.addRule') }}
									</button>
								</div>

								<div class="bg-base-200 rounded-xl border border-white/5 overflow-hidden">
									<div class="max-h-[300px] overflow-y-auto">
										<div v-for="(rule, index) in parsedConfig.rules?.slice(0, 50)" :key="index"
											class="flex items-center justify-between p-3 border-b border-white/5 hover:bg-base-300/30 transition-colors group">
											<span class="font-mono text-[11px] opacity-70 truncate pr-4">{{ rule
											}}</span>
											<button
												class="btn btn-xs btn-square btn-ghost text-error opacity-0 group-hover:opacity-100 transition-opacity"
												@click="deleteRule(Number(index))">
												<i class="fa-solid fa-xmark"></i>
											</button>
										</div>
										<div v-if="parsedConfig.rules?.length > 50"
											class="p-3 text-center text-[10px] opacity-30 italic">
											{{ t('profiles.moreRules', { count: parsedConfig.rules.length - 50 }) }}
										</div>
									</div>
									<div v-if="!parsedConfig.rules?.length"
										class="py-12 flex flex-col items-center justify-center opacity-20">
										<i class="fa-solid fa-route text-3xl mb-2"></i>
										<span class="text-xs">{{ t('profiles.noRules') }}</span>
									</div>
								</div>
							</section>
						</div>

				<!-- YAML Editor -->
				<div v-if="editMode === 'yaml'" class="flex-1 flex flex-col min-h-0">
							<Codemirror
								v-model="currentContent"
								:extensions="yamlExtensions"
								:autofocus="true"
								:indent-with-tab="true"
								:tab-size="2"
								placeholder="YAML content here..."
								class="flex-1 w-full bg-neutral/30 overflow-hidden"
							/>
				</div>
					</div>
				</div>

				<!-- Empty State -->
				<div v-else class="flex-1 flex flex-col items-center justify-center opacity-20">
					<i class="fa-solid fa-file-circle-plus text-6xl mb-4"></i>
					<span class="text-lg font-medium">{{ t('profiles.selectOrCreate') }}</span>
				</div>
			</div>
		</div>

		<!-- New Profile Modal -->
		<div v-if="showNewProfileModal" class="fixed inset-0 z-[1100] flex items-center justify-center p-4">
			<div class="absolute inset-0 bg-black/40 backdrop-blur-sm" @click="showNewProfileModal = false"></div>
			<div
				class="modal-box relative bg-base-100 border border-white/10 shadow-2xl animate-in zoom-in duration-200">
				<h3 class="font-bold text-lg mb-6">{{ t('profiles.addProfile') }}</h3>
				<div class="space-y-4">
					<div class="form-control w-full">
						<label class="label"><span class="label-text">{{ t('profiles.profileName') }}</span></label>
						<input type="text" v-model="newProfileName"
							class="input input-bordered w-full bg-base-200 border-none focus:outline-none"
							:placeholder="t('profiles.profileName')" />
					</div>
					<div class="form-control w-full">
						<label class="label">
							<span class="label-text">{{ t('profiles.profileSource') }}</span>
						</label>
						<div class="join w-full">
							<button type="button" class="btn join-item"
								:class="newProfileSource === 'empty' ? 'btn-primary' : 'btn-ghost bg-base-200'"
								@click="handleSwitchSource('empty')">
								{{ t('profiles.profileSourceEmpty') }}
							</button>
							<button type="button" class="btn join-item"
								:class="newProfileSource === 'url' ? 'btn-primary' : 'btn-ghost bg-base-200'"
								@click="handleSwitchSource('url')">
								{{ t('profiles.profileSourceUrl') }}
							</button>
							<button type="button" class="btn join-item"
								:class="newProfileSource === 'file' ? 'btn-primary' : 'btn-ghost bg-base-200'"
								@click="handleSwitchSource('file')">
								{{ t('profiles.profileSourceFile') }}
							</button>
						</div>
					</div>
					<div class="form-control w-full" v-if="newProfileSource === 'url'">
						<label class="label"><span class="label-text">{{ t('profiles.profileUrl') }}</span></label>
						<input type="text" v-model="newProfileUrl"
							class="input input-bordered w-full bg-base-200 border-none focus:outline-none"
							placeholder="https://example.com/clash.yaml" />
					</div>
					<div class="form-control w-full" v-if="newProfileSource === 'file'">
						<label class="label"><span class="label-text">{{ t('profiles.profilePath') }}</span></label>
						<div class="join w-full">
							<input type="text" v-model="newProfilePath"
								class="input input-bordered join-item w-full bg-base-200 border-none focus:outline-none"
								:placeholder="t('profiles.profilePathHint')" />
							<button type="button" class="btn join-item btn-ghost bg-base-200"
								@click="handlePickProfileFile">
								{{ t('profiles.chooseFile') }}
							</button>
						</div>
					</div>
				</div>
				<div class="modal-action">
					<button class="btn btn-ghost" @click="showNewProfileModal = false">{{ t('profiles.cancel') }}</button>
					<button class="btn btn-primary px-8" @click="handleCreateProfile">{{ t('profiles.create') }}</button>
				</div>
			</div>
		</div>

		<!-- Proxy Edit Modal -->
		<div v-if="showProxyModal" class="fixed inset-0 z-[1100] flex items-center justify-center p-4">
			<div class="absolute inset-0 bg-black/40 backdrop-blur-sm" @click="showProxyModal = false"></div>
			<div
				class="modal-box relative max-w-2xl bg-base-100 border border-white/10 shadow-2xl animate-in zoom-in duration-200">
				<h3 class="font-bold text-lg mb-6">{{ t('profiles.addProxyTitle', { mode: editProxyIndex === -1 ? t('actions.add') : t('actions.edit') }) }}</h3>

				<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
					<div class="form-control w-full">
						<label class="label"><span class="label-text text-xs">{{ t('profiles.proxyName') }} *</span></label>
						<input type="text" v-model="editProxy.name"
							class="input input-bordered input-sm bg-base-200 border-none focus:outline-none" />
					</div>
					<div class="form-control w-full">
						<label class="label"><span class="label-text text-xs">{{ t('profiles.proxyType') }} *</span></label>
						<select v-model="editProxy.type"
							class="select select-bordered select-sm bg-base-200 border-none focus:outline-none">
							<option v-for="t in proxyTypes" :key="t" :value="t">{{ t.toUpperCase() }}</option>
						</select>
					</div>
					<div class="form-control w-full">
						<label class="label"><span class="label-text text-xs">{{ t('profiles.proxyServer') }} *</span></label>
						<input type="text" v-model="editProxy.server"
							class="input input-bordered input-sm bg-base-200 border-none focus:outline-none" />
					</div>
					<div class="form-control w-full">
						<label class="label"><span class="label-text text-xs">{{ t('profiles.proxyPort') }} *</span></label>
						<input type="number" v-model.number="editProxy.port"
							class="input input-bordered input-sm bg-base-200 border-none focus:outline-none" />
					</div>

					<div class="form-control w-full" v-if="['ss', 'trojan'].includes(editProxy.type)">
						<label class="label"><span class="label-text text-xs">{{ t('profiles.proxyPassword') }}</span></label>
						<input type="text" v-model="editProxy.password"
							class="input input-bordered input-sm bg-base-200 border-none focus:outline-none" />
					</div>
					<div class="form-control w-full" v-if="isVmessOrVless">
						<label class="label"><span class="label-text text-xs">{{ t('profiles.proxyUuid') }}</span></label>
						<input type="text" v-model="editProxy.uuid"
							class="input input-bordered input-sm bg-base-200 border-none focus:outline-none" />
					</div>
					<div class="form-control w-full" v-if="isVmess">
						<label class="label"><span class="label-text text-xs">{{ t('profiles.proxyAlterId') }}</span></label>
						<input type="number" v-model.number="editProxy.alterId"
							class="input input-bordered input-sm bg-base-200 border-none focus:outline-none" />
					</div>
					<div class="form-control w-full" v-if="isVmess">
						<label class="label"><span class="label-text text-xs">{{ t('profiles.proxyVmessCipher') }}</span></label>
						<select v-model="editProxy.cipher"
							class="select select-bordered select-sm bg-base-200 border-none focus:outline-none">
							<option>auto</option>
							<option>aes-128-gcm</option>
							<option>chacha20-poly1305</option>
							<option>none</option>
						</select>
					</div>
					<div class="form-control w-full" v-if="editProxy.type === 'ss'">
						<label class="label"><span class="label-text text-xs">{{ t('profiles.proxyCipher') }}</span></label>
						<select v-model="editProxy.cipher"
							class="select select-bordered select-sm bg-base-200 border-none focus:outline-none">
							<option>aes-256-gcm</option>
							<option>aes-128-gcm</option>
							<option>chacha20-ietf-poly1305</option>
							<option>2022-blake3-aes-256-gcm</option>
						</select>
					</div>
					<div class="form-control w-full">
						<label class="label"><span class="label-text text-xs">{{ t('profiles.proxySni') }}</span></label>
						<input type="text" v-model="editProxy.sni"
							class="input input-bordered input-sm bg-base-200 border-none focus:outline-none"
							placeholder="sni.example.com" />
					</div>

					<div class="flex gap-8 mt-4 md:col-span-2">
						<div class="form-control">
							<label class="label cursor-pointer gap-3">
								<span class="label-text text-xs">{{ t('profiles.proxyUdp') }}</span>
								<input type="checkbox" class="toggle toggle-primary toggle-xs"
									v-model="editProxy.udp" />
							</label>
						</div>
						<div class="form-control">
							<label class="label cursor-pointer gap-3">
								<span class="label-text text-xs">{{ t('profiles.proxyTls') }}</span>
								<input type="checkbox" class="toggle toggle-primary toggle-xs"
									v-model="editProxy.tls" />
							</label>
						</div>
					</div>
				</div>

				<div class="modal-action mt-8">
					<button class="btn btn-ghost" @click="showProxyModal = false">{{ t('profiles.cancel') }}</button>
					<button class="btn btn-primary px-8" @click="saveProxy">{{ t('profiles.saveProxy') }}</button>
				</div>
			</div>
		</div>

		<!-- Proxy Group Edit Modal -->
		<div v-if="showGroupModal" class="fixed inset-0 z-[1100] flex items-center justify-center p-4">
			<div class="absolute inset-0 bg-black/40 backdrop-blur-sm" @click="showGroupModal = false"></div>
			<div
				class="modal-box relative max-w-2xl bg-base-100 border border-white/10 shadow-2xl animate-in zoom-in duration-200">
				<h3 class="font-bold text-lg mb-6">{{ editGroupIndex === -1 ? t('proxies.addGroup') : t('proxies.editGroup') }}</h3>

				<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
					<div class="form-control w-full">
						<label class="label"><span class="label-text text-xs">{{ t('proxies.groupName') }} *</span></label>
						<input type="text" v-model="editGroup.name"
							class="input input-bordered input-sm bg-base-200 border-none focus:outline-none" />
					</div>
					<div class="form-control w-full">
						<label class="label"><span class="label-text text-xs">{{ t('proxies.groupType') }} *</span></label>
						<select v-model="editGroup.type"
							class="select select-bordered select-sm bg-base-200 border-none focus:outline-none">
							<option v-for="t in groupTypes" :key="t" :value="t">{{ t.toUpperCase() }}</option>
						</select>
					</div>
					
					<div class="form-control w-full" v-if="['url-test', 'fallback', 'load-balance'].includes(editGroup.type)">
						<label class="label"><span class="label-text text-xs">{{ t('proxies.testUrl') }}</span></label>
						<input type="text" v-model="editGroup.url"
							class="input input-bordered input-sm bg-base-200 border-none focus:outline-none" />
					</div>
					<div class="form-control w-full" v-if="['url-test', 'fallback', 'load-balance'].includes(editGroup.type)">
						<label class="label"><span class="label-text text-xs">{{ t('proxies.intervalSeconds') }}</span></label>
						<input type="number" v-model.number="editGroup.interval"
							class="input input-bordered input-sm bg-base-200 border-none focus:outline-none" />
					</div>

					<div class="form-control w-full md:col-span-2">
						<label class="label">
							<span class="label-text text-xs">{{ t('proxies.selectProxies') }}</span>
							<span class="label-text-alt text-xs">{{ editGroup.proxies.length }} {{ t('common.selected', { count: '' }).replace(': count', '') }}</span>
						</label>
						<div class="bg-base-200 rounded-lg p-3 max-h-64 overflow-y-auto">
							<div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
								<label v-for="proxy in parsedConfig.proxies" :key="proxy.name"
									class="label cursor-pointer justify-start gap-3 p-2 rounded hover:bg-base-300/50">
									<input type="checkbox" :checked="editGroup.proxies.includes(proxy.name)"
										@change="toggleProxyInGroup(proxy.name)"
										class="checkbox checkbox-primary checkbox-sm" />
									<span class="label-text text-xs">{{ proxy.name }}</span>
								</label>
								<!-- Other groups as options -->
								<label v-for="(group, idx) in parsedConfig['proxy-groups']" :key="idx"
									v-show="idx !== editGroupIndex"
									class="label cursor-pointer justify-start gap-3 p-2 rounded hover:bg-base-300/50">
									<input type="checkbox" :checked="editGroup.proxies.includes(group.name)"
										@change="toggleProxyInGroup(group.name)"
										class="checkbox checkbox-primary checkbox-sm" />
									<span class="label-text text-xs font-bold">{{ group.name }}</span>
								</label>
							</div>
						</div>
					</div>
				</div>

				<div class="modal-action mt-8">
					<button class="btn btn-ghost" @click="showGroupModal = false">{{ t('actions.cancel') }}</button>
					<button class="btn btn-primary px-8" @click="saveGroup">{{ t('actions.save') }}</button>
				</div>
			</div>
		</div>
	</div>
</template>

<style scoped>
/* 确保 CodeMirror 填充整个容器并处理内部滚动 */
:deep(.cm-editor) {
	height: 100% !important;
	outline: none !important;
}

:deep(.cm-scroller) {
	overflow: auto !important;
	height: 100% !important;
}

/* 自定义滚动条样式 */
:deep(.cm-scroller::-webkit-scrollbar) {
	width: 6px;
	height: 6px;
}

:deep(.cm-scroller::-webkit-scrollbar-track) {
	background: transparent;
}

:deep(.cm-scroller::-webkit-scrollbar-thumb) {
	background: rgba(255, 255, 255, 0.1);
	border-radius: 10px;
}

:deep(.cm-scroller::-webkit-scrollbar-thumb:hover) {
	background: rgba(255, 255, 255, 0.2);
}
</style>
