import { ref } from 'vue';

export interface Toast {
    id: number;
    message: string;
    type: 'info' | 'success' | 'warning' | 'error';
    duration?: number;
}

const toasts = ref<Toast[]>([]);
let nextId = 1;

export function useToast() {
    const show = (message: string, type: Toast['type'] = 'info', duration = 3000) => {
        const id = nextId++;
        const toast: Toast = { id, message, type, duration };
        
        toasts.value.push(toast);
        
        if (duration > 0) {
            setTimeout(() => {
                remove(id);
            }, duration);
        }
        
        return id;
    };
    
    const remove = (id: number) => {
        const index = toasts.value.findIndex(t => t.id === id);
        if (index > -1) {
            toasts.value.splice(index, 1);
        }
    };
    
    const success = (message: string, duration?: number) => show(message, 'success', duration);
    const error = (message: string, duration?: number) => show(message, 'error', duration);
    const warning = (message: string, duration?: number) => show(message, 'warning', duration);
    const info = (message: string, duration?: number) => show(message, 'info', duration);
    
    return {
        toasts,
        show,
        remove,
        success,
        error,
        warning,
        info,
    };
}
