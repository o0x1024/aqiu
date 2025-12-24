<script setup lang="ts">
import { ref, computed } from 'vue';
import { useMihomo } from '../composables/useMihomo';
import { useI18n } from '../composables/useI18n';
import { useToast } from '../composables/useToast';

const { rules, config, updateConfig, proxies } = useMihomo();
const { t } = useI18n();
const { success, error, warning } = useToast();

const searchRules = ref('');
const showAddRuleModal = ref(false);

// Rule form
const newRule = ref({
	type: 'DOMAIN-SUFFIX',
	payload: '',
	target: 'DIRECT'
});

const ruleTypes = [
	'DOMAIN',
	'DOMAIN-SUFFIX',
	'DOMAIN-KEYWORD',
	'IP-CIDR',
	'IP-CIDR6',
	'SRC-IP-CIDR',
	'GEOIP',
	'DST-PORT',
	'SRC-PORT',
	'PROCESS-NAME',
	'MATCH'
];

// Get available proxy groups and special targets
const proxyTargets = computed(() => {
	const targets = ['DIRECT', 'REJECT'];
	const groups = Object.values(proxies.value)
		.filter(p => p.type === 'Selector' || p.type === 'URLTest' || p.type === 'Fallback')
		.map(p => p.name);
	return [...targets, ...groups];
});

const filteredRules = computed(() => {
	if (!searchRules.value) return rules.value.slice(0, 100);
	const query = searchRules.value.toLowerCase();
	return rules.value.filter(r =>
		r.type.toLowerCase().includes(query) ||
		r.payload.toLowerCase().includes(query) ||
		r.proxy.toLowerCase().includes(query)
	).slice(0, 100);
});

const openAddRuleModal = () => {
	newRule.value = {
		type: 'DOMAIN-SUFFIX',
		payload: '',
		target: proxyTargets.value[0] || 'DIRECT'
	};
	showAddRuleModal.value = true;
};

const handleAddRule = async () => {
	if (!newRule.value.payload.trim()) {
		warning(t('rules.payloadRequired'));
		return;
	}

	try {
		// Format: TYPE,PAYLOAD,TARGET
		const ruleString = `${newRule.value.type},${newRule.value.payload.trim()},${newRule.value.target}`;

		// Get current rules from config
		const currentRules = config.value?.rules || [];
		const updatedRules = [ruleString, ...currentRules];

		await updateConfig({ rules: updatedRules });

		showAddRuleModal.value = false;
		success(t('rules.addedSuccess'));
	} catch (e) {
		error(t('rules.addFailed', { error: String(e) }));
	}
};

const getPlaceholder = () => {
	switch (newRule.value.type) {
		case 'DOMAIN':
			return 'example.com';
		case 'DOMAIN-SUFFIX':
			return 'google.com';
		case 'DOMAIN-KEYWORD':
			return 'google';
		case 'IP-CIDR':
			return '192.168.1.0/24';
		case 'IP-CIDR6':
			return '2001:db8::/32';
		case 'GEOIP':
			return 'CN';
		case 'DST-PORT':
			return '80';
		case 'SRC-PORT':
			return '7890';
		case 'PROCESS-NAME':
			return 'chrome.exe';
		case 'MATCH':
			return 'no-resolve';
		default:
			return '';
	}
};
</script>

<template>
	<div class="space-y-6 animate-in slide-in-from-bottom-4 duration-500">
		<div class="flex items-center justify-between">
			<h1 class="text-3xl font-bold tracking-tight">{{ t('rules.title') }}</h1>
			<div class="flex items-center gap-3">
				<div class="badge badge-ghost">{{ t('common.total', { count: rules.length }) }}</div>
				<button class="btn btn-sm btn-primary" @click="openAddRuleModal">
					<i class="fa-solid fa-plus"></i> {{ t('rules.addRule') }}
				</button>
			</div>
		</div>

		<div class="join w-full shadow-sm">
			<div class="join-item bg-base-200 flex items-center px-4">
				<i class="fa-solid fa-search opacity-50"></i>
			</div>
			<input type="text" v-model="searchRules" :placeholder="t('rules.searchPlaceholder')"
				class="input input-bordered join-item w-full bg-base-200 border-none focus:outline-none" />
		</div>

		<div class="overflow-x-auto rounded-xl border border-white/5 bg-base-200 shadow-sm">
			<table class="table table-zebra table-sm">
				<thead>
					<tr class="bg-base-300/50">
						<th class="py-3">{{ t('rules.tableType') }}</th>
						<th>{{ t('rules.tablePayload') }}</th>
						<th>{{ t('rules.tableTarget') }}</th>
					</tr>
				</thead>
				<tbody>
					<tr v-for="(rule, i) in filteredRules" :key="i" class="hover">
						<td class="font-mono text-[11px] opacity-70">{{ rule.type }}</td>
						<td class="max-w-xs truncate text-[11px]">{{ rule.payload }}</td>
						<td><span class="badge badge-sm badge-outline opacity-70">{{ rule.proxy }}</span>
						</td>
					</tr>
				</tbody>
			</table>
		</div>

		<!-- Add Rule Modal -->
		<div v-if="showAddRuleModal" class="fixed inset-0 z-[1100] flex items-center justify-center p-4">
			<div class="absolute inset-0 bg-black/40 backdrop-blur-sm" @click="showAddRuleModal = false"></div>
			<div
				class="modal-box relative bg-base-100 border border-white/10 shadow-2xl animate-in zoom-in duration-200">
				<h3 class="font-bold text-lg mb-6">{{ t('rules.addTitle') }}</h3>

				<div class="space-y-4">
					<div class="form-control w-full">
						<label class="label">
							<span class="label-text text-xs font-medium">{{ t('rules.ruleType') }}</span>
						</label>
						<select v-model="newRule.type"
							class="select select-bordered select-sm bg-base-200 border-none focus:outline-none">
							<option v-for="type in ruleTypes" :key="type" :value="type">{{ type }}</option>
						</select>
						<label class="label">
							<span class="label-text-alt opacity-50">{{ t('rules.ruleTypeHint') }}</span>
						</label>
					</div>

					<div class="form-control w-full">
						<label class="label">
							<span class="label-text text-xs font-medium">{{ t('rules.payload') }}</span>
						</label>
						<input type="text" v-model="newRule.payload" :placeholder="getPlaceholder()"
							class="input input-bordered input-sm bg-base-200 border-none focus:outline-none" />
						<label class="label">
							<span class="label-text-alt opacity-50">{{ t('rules.payloadHint') }}</span>
						</label>
					</div>

					<div class="form-control w-full">
						<label class="label">
							<span class="label-text text-xs font-medium">{{ t('rules.target') }}</span>
						</label>
						<select v-model="newRule.target"
							class="select select-bordered select-sm bg-base-200 border-none focus:outline-none">
							<option v-for="target in proxyTargets" :key="target" :value="target">{{ target }}</option>
						</select>
						<label class="label">
							<span class="label-text-alt opacity-50">{{ t('rules.targetHint') }}</span>
						</label>
					</div>
				</div>

				<div class="modal-action">
					<button class="btn btn-ghost" @click="showAddRuleModal = false">{{ t('actions.cancel') }}</button>
					<button class="btn btn-primary px-8" @click="handleAddRule">{{ t('rules.addRule') }}</button>
				</div>
			</div>
		</div>
	</div>
</template>
