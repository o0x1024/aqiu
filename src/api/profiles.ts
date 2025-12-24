import { invoke } from '@tauri-apps/api/core';

// ========== Types ==========

export interface Profile {
    id: string;
    name: string;
    url: string | null;
    file_path: string;
    updated_at: string;
    is_active: boolean;
}

// ========== Profile Management ==========

export async function listProfiles(): Promise<Profile[]> {
    return await invoke<Profile[]>('list_profiles');
}

export async function getActiveProfile(): Promise<Profile | null> {
    return await invoke<Profile | null>('get_active_profile');
}

export async function createProfile(name: string, url?: string): Promise<Profile> {
    return await invoke<Profile>('create_profile', { name, url });
}

export async function createProfileFromPath(name: string, path: string, url?: string): Promise<Profile> {
    return await invoke<Profile>('create_profile_from_path', { name, path, url });
}

export async function deleteProfile(id: string): Promise<void> {
    return await invoke('delete_profile', { id });
}

export async function setActiveProfile(id: string): Promise<void> {
    return await invoke('set_active_profile', { id });
}

export async function getProfileContent(id: string): Promise<string> {
    return await invoke<string>('get_profile_content', { id });
}

export async function saveProfileContent(id: string, content: string): Promise<void> {
    return await invoke('save_profile_content', { id, content });
}

export async function renameProfile(id: string, newName: string): Promise<void> {
    return await invoke('rename_profile', { id, newName });
}

export async function updateProfileFromUrl(id: string): Promise<string> {
    return await invoke<string>('update_profile_from_url', { id });
}

export async function parseConfig(content: string): Promise<any> {
    return await invoke('parse_config', { content });
}

export async function saveConfigObj(id: string, config: any): Promise<void> {
    return await invoke('save_config_obj', { id, config });
}

export async function addProxyToProfile(id: string, proxy: any): Promise<void> {
    return await invoke('add_proxy_to_profile', { id, proxy });
}

export async function parseProxyUrl(url: string): Promise<any> {
    return await invoke('parse_proxy_url', { url });
}

export async function getActiveProfilePath(): Promise<string | null> {
    return await invoke<string | null>('get_active_profile_path');
}
