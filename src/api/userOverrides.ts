import { invoke } from '@tauri-apps/api/core';

export interface TunOverride {
    enable?: boolean;
}

export interface UserConfigOverrides {
    port?: number;
    'socks-port'?: number;
    'mixed-port'?: number;
    'redir-port'?: number;
    'tproxy-port'?: number;
    'allow-lan'?: boolean;
    'external-controller'?: string;
    tun?: TunOverride;
}

export async function setUserOverride(key: string, value: any): Promise<void> {
    return await invoke('set_user_override', { key, value });
}

export async function getUserOverrides(): Promise<UserConfigOverrides> {
    return await invoke<UserConfigOverrides>('get_user_overrides');
}

export async function clearUserOverrides(): Promise<void> {
    return await invoke('clear_user_overrides');
}
