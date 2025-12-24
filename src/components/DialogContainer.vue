<script setup lang="ts">
import { nextTick, watch } from 'vue';
import { useDialog } from '../composables/useDialog';

const { state, submit, cancel } = useDialog();

const focusInput = async () => {
    await nextTick();
    const el = document.querySelector<HTMLInputElement>('[data-dialog-input]');
    if (el) {
        el.focus();
        el.select();
    }
};

watch(
    () => [state.open, state.mode],
    ([open, mode]) => {
        if (open && mode === 'prompt') {
            focusInput();
        }
    }
);
</script>

<template>
    <div v-if="state.open" class="fixed inset-0 z-[1200] flex items-center justify-center p-4">
        <div class="absolute inset-0 bg-black/40 backdrop-blur-sm" @click="cancel"></div>
        <div class="modal-box relative max-w-lg bg-base-100 border border-white/10 shadow-2xl animate-in zoom-in duration-200">
            <h3 class="font-bold text-lg mb-3">{{ state.title }}</h3>
            <p class="text-sm opacity-80 whitespace-pre-line">{{ state.message }}</p>

            <div v-if="state.mode === 'prompt'" class="mt-5 space-y-2">
                <label v-if="state.inputLabel" class="label">
                    <span class="label-text text-xs">{{ state.inputLabel }}</span>
                </label>
                <input
                    data-dialog-input
                    type="text"
                    v-model="state.inputValue"
                    :placeholder="state.inputPlaceholder"
                    class="input input-bordered w-full bg-base-200 border-none focus:outline-none"
                />
            </div>

            <div class="modal-action">
                <button class="btn btn-ghost" @click="cancel">{{ state.cancelText }}</button>
                <button class="btn btn-primary px-6" @click="submit">{{ state.confirmText }}</button>
            </div>
        </div>
    </div>
</template>
