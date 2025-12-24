<script setup lang="ts">
import { useToast } from '../composables/useToast';

const { toasts, remove } = useToast();

const getAlertClass = (type: string) => {
	switch (type) {
		case 'success':
			return 'alert-success';
		case 'error':
			return 'alert-error';
		case 'warning':
			return 'alert-warning';
		case 'info':
		default:
			return 'alert-info';
	}
};

const getIcon = (type: string) => {
	switch (type) {
		case 'success':
			return 'fa-circle-check';
		case 'error':
			return 'fa-circle-xmark';
		case 'warning':
			return 'fa-triangle-exclamation';
		case 'info':
		default:
			return 'fa-circle-info';
	}
};
</script>

<template>
	<div class="toast toast-top toast-end z-50">
		<TransitionGroup name="toast">
			<div v-for="toast in toasts" :key="toast.id" class="alert shadow-lg" :class="getAlertClass(toast.type)">
				<div class="flex items-center gap-3">
					<i class="fa-solid text-lg" :class="getIcon(toast.type)"></i>
					<span>{{ toast.message }}</span>
				</div>
				<button class="btn btn-sm btn-ghost btn-circle" @click="remove(toast.id)">
					<i class="fa-solid fa-xmark"></i>
				</button>
			</div>
		</TransitionGroup>
	</div>
</template>

<style scoped>
.toast-enter-active,
.toast-leave-active {
	transition: all 0.3s ease;
}

.toast-enter-from {
	opacity: 0;
	transform: translateX(100%);
}

.toast-leave-to {
	opacity: 0;
	transform: translateX(100%) scale(0.8);
}

.toast-move {
	transition: transform 0.3s ease;
}
</style>
