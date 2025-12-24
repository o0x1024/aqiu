import { reactive } from 'vue';

type DialogMode = 'confirm' | 'prompt';

interface DialogState {
    open: boolean;
    mode: DialogMode;
    title: string;
    message: string;
    confirmText: string;
    cancelText: string;
    inputLabel: string;
    inputPlaceholder: string;
    inputValue: string;
}

interface ConfirmOptions {
    title: string;
    message: string;
    confirmText: string;
    cancelText: string;
}

interface PromptOptions extends ConfirmOptions {
    inputLabel?: string;
    inputPlaceholder?: string;
    defaultValue?: string;
}

const state = reactive<DialogState>({
    open: false,
    mode: 'confirm',
    title: '',
    message: '',
    confirmText: '',
    cancelText: '',
    inputLabel: '',
    inputPlaceholder: '',
    inputValue: '',
});

let resolver: any = null;

const close = (value: boolean | string | null) => {
    state.open = false;
    const resolve = resolver;
    resolver = null;
    resolve?.(value);
};

const confirm = (options: ConfirmOptions) => {
    state.mode = 'confirm';
    state.open = true;
    state.title = options.title;
    state.message = options.message;
    state.confirmText = options.confirmText;
    state.cancelText = options.cancelText;
    state.inputLabel = '';
    state.inputPlaceholder = '';
    state.inputValue = '';

    return new Promise<boolean>((resolve) => {
        resolver = resolve;
    });
};

const prompt = (options: PromptOptions) => {
    state.mode = 'prompt';
    state.open = true;
    state.title = options.title;
    state.message = options.message;
    state.confirmText = options.confirmText;
    state.cancelText = options.cancelText;
    state.inputLabel = options.inputLabel || '';
    state.inputPlaceholder = options.inputPlaceholder || '';
    state.inputValue = options.defaultValue || '';

    return new Promise<string | null>((resolve) => {
        resolver = resolve;
    });
};

const submit = () => {
    if (state.mode === 'confirm') {
        close(true);
        return;
    }
    close(state.inputValue);
};

const cancel = () => {
    close(state.mode === 'confirm' ? false : null);
};

export function useDialog() {
    return {
        state,
        confirm,
        prompt,
        submit,
        cancel,
    };
}
