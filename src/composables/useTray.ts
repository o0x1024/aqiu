import { watch } from 'vue';
import { useStorage } from '@vueuse/core';
import { useCore } from './useCore';
import { useMihomo } from './useMihomo';
import * as tauriApi from '../api/tauri';

export function useTray() {
    const showTraySpeed = useStorage('aqiu-show-tray-speed', false);
    const { systemProxyEnabled } = useCore();
    const { config, traffic } = useMihomo();

    const formatSpeed = (bytes: number) => {
        if (bytes < 1024) return `${bytes} B/s`;
        if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB/s`;
        return `${(bytes / 1024 / 1024).toFixed(1)} MB/s`;
    };

    // Update tray menu state
    watch([systemProxyEnabled, () => config.value?.tun?.enable, () => config.value?.mode], async ([proxy, tun, mode]) => {
        try {
            await tauriApi.updateTrayState(
                !!proxy,
                !!tun,
                (mode || 'rule') as string
            );
        } catch (e) {
            console.error("Failed to update tray state", e);
        }
    }, { immediate: true });

    // Speed icon rendering (macOS multi-line workaround)
    let iconImg: HTMLImageElement | null = null;
    const loadIcon = () => {
        return new Promise<HTMLImageElement>((resolve, reject) => {
            if (iconImg) return resolve(iconImg);
            const img = new Image();
            img.src = '/tray-icon.png';
            const timeout = setTimeout(() => {
                img.onload = null;
                img.onerror = null;
                reject(new Error("Tray icon load timeout"));
            }, 5000);

            img.onload = () => {
                clearTimeout(timeout);
                iconImg = img;
                resolve(img);
            };
            img.onerror = () => {
                clearTimeout(timeout);
                reject(new Error("Failed to load tray icon image"));
            };
        });
    };

    const renderSpeedIcon = async (up: number, down: number) => {
        const canvas = document.createElement('canvas');
        const scale = 2; // retina support
        const canvasWidth = 80; // slightly more compact
        const canvasHeight = 22;

        canvas.width = canvasWidth * scale;
        canvas.height = canvasHeight * scale;
        const ctx = canvas.getContext('2d');
        if (!ctx) return null;

        ctx.scale(scale, scale);

        // 1. Draw original icon on the far left
        const img = await loadIcon();
        const iconSize = 22;
        const iconY = (canvasHeight - iconSize) / 2;
        ctx.drawImage(img, 0, iconY, iconSize, iconSize);

        // 2. Setup text style
        ctx.fillStyle = 'white';
        // Arrow font can be different if needed, but standard bold is usually fine
        const fontStr = '10px -apple-system, BlinkMacSystemFont, "Helvetica Neue", sans-serif';
        ctx.font = fontStr;

        const upStr = formatSpeed(up);
        const downStr = formatSpeed(down);

        // 3. Draw arrows and text
        // Layout: [LOGO] [Arrow] [Speed]
        const textX = 24; // Start after logo

        // Upload (Top)
        ctx.textAlign = 'left';
        ctx.fillText('↑', textX, 9);
        ctx.textAlign = 'right';
        ctx.fillText(upStr, canvasWidth - 2, 9);

        // Download (Bottom)
        ctx.textAlign = 'left';
        ctx.fillText('↓', textX, 22);
        ctx.textAlign = 'right';
        ctx.fillText(downStr, canvasWidth - 2, 22);

        return new Promise<Uint8Array | null>((resolve) => {
            canvas.toBlob(async (blob) => {
                if (blob) {
                    const buffer = await blob.arrayBuffer();
                    resolve(new Uint8Array(buffer));
                } else {
                    resolve(null);
                }
            }, 'image/png');
        });
    };

    // Update tray title/icon with speed (macOS)
    watch([() => traffic.up, () => traffic.down, showTraySpeed], async ([up, down, show]) => {
        if (!show) {
            try {
                await tauriApi.updateTrayTitle("");
                await tauriApi.restoreTrayIcon();
            } catch { /* Ignore errors when restoring tray icon */ }
            return;
        }

        try {
            const iconBytes = await renderSpeedIcon(up as number, down as number);
            if (iconBytes) {
                await tauriApi.updateTrayIcon(iconBytes);
                // Clear title as we now use the icon for information
                await tauriApi.updateTrayTitle("");
            }
        } catch (e) {
            console.error("Failed to update tray icon", e);
        }
    }, { immediate: true });
}
