import { ref, onMounted } from 'vue';
import * as profilesApi from '../api/profiles';

export interface Profile {
    id: string;
    name: string;
    url: string | null;
    file_path: string;
    updated_at: string;
    is_active: boolean;
}

export function useProfiles() {
    const profiles = ref<Profile[]>([]);
    const activeProfile = ref<Profile | null>(null);
    const currentContent = ref<string>('');
    const isLoading = ref(false);
    const error = ref<string | null>(null);

    const fetchProfiles = async () => {
        try {
            profiles.value = await profilesApi.listProfiles();
            activeProfile.value = await profilesApi.getActiveProfile();
        } catch (e) {
            console.error('Failed to fetch profiles:', e);
            error.value = String(e);
        }
    };

    const createProfile = async (name: string, url?: string, path?: string) => {
        isLoading.value = true;
        error.value = null;
        try {
            const profile = path
                ? await profilesApi.createProfileFromPath(name, path, url)
                : await profilesApi.createProfile(name, url);
            profiles.value.push(profile);
            if (profile.is_active) {
                activeProfile.value = profile;
            }
            return profile;
        } catch (e) {
            error.value = String(e);
            throw e;
        } finally {
            isLoading.value = false;
        }
    };

    const deleteProfile = async (id: string) => {
        isLoading.value = true;
        error.value = null;
        try {
            await profilesApi.deleteProfile(id);
            profiles.value = profiles.value.filter(p => p.id !== id);
            if (activeProfile.value?.id === id) {
                activeProfile.value = profiles.value.find(p => p.is_active) || null;
            }
        } catch (e) {
            error.value = String(e);
            throw e;
        } finally {
            isLoading.value = false;
        }
    };

    const setActive = async (id: string) => {
        isLoading.value = true;
        error.value = null;
        try {
            await profilesApi.setActiveProfile(id);
            profiles.value.forEach(p => p.is_active = p.id === id);
            activeProfile.value = profiles.value.find(p => p.id === id) || null;
        } catch (e) {
            error.value = String(e);
            throw e;
        } finally {
            isLoading.value = false;
        }
    };

    const loadContent = async (id: string) => {
        isLoading.value = true;
        error.value = null;
        try {
            currentContent.value = await profilesApi.getProfileContent(id);
        } catch (e) {
            error.value = String(e);
            throw e;
        } finally {
            isLoading.value = false;
        }
    };

    const saveContent = async (id: string, content: string) => {
        isLoading.value = true;
        error.value = null;
        try {
            await profilesApi.saveProfileContent(id, content);
            currentContent.value = content;
            // Update timestamp
            const profile = profiles.value.find(p => p.id === id);
            if (profile) {
                profile.updated_at = new Date().toLocaleString();
            }
        } catch (e) {
            error.value = String(e);
            throw e;
        } finally {
            isLoading.value = false;
        }
    };

    const renameProfile = async (id: string, newName: string) => {
        isLoading.value = true;
        error.value = null;
        try {
            await profilesApi.renameProfile(id, newName);
            const profile = profiles.value.find(p => p.id === id);
            if (profile) {
                profile.name = newName;
            }
        } catch (e) {
            error.value = String(e);
            throw e;
        } finally {
            isLoading.value = false;
        }
    };

    const updateFromUrl = async (id: string) => {
        isLoading.value = true;
        error.value = null;
        try {
            await profilesApi.updateProfileFromUrl(id);
            await fetchProfiles();
        } catch (e) {
            error.value = String(e);
            throw e;
        } finally {
            isLoading.value = false;
        }
    };

    const addProxy = async (id: string, proxy: any) => {
        isLoading.value = true;
        error.value = null;
        try {
            await profilesApi.addProxyToProfile(id, proxy);
            await loadContent(id);
        } catch (e) {
            error.value = String(e);
            throw e;
        } finally {
            isLoading.value = false;
        }
    };

    const saveConfigObj = async (id: string, config: any) => {
        isLoading.value = true;
        error.value = null;
        try {
            await profilesApi.saveConfigObj(id, config);
            await loadContent(id);
        } catch (e) {
            error.value = String(e);
            throw e;
        } finally {
            isLoading.value = false;
        }
    };

    onMounted(() => {
        fetchProfiles();
    });

    return {
        profiles,
        activeProfile,
        currentContent,
        isLoading,
        error,
        fetchProfiles,
        createProfile,
        deleteProfile,
        setActive,
        loadContent,
        saveContent,
        renameProfile,
        updateFromUrl,
        addProxy,
        saveConfigObj,
    };
}
