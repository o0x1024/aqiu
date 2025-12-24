import { ref, watch } from 'vue';

const themeOptions = [
    'aqiu', 'light', 'dark', 'corporate', 'business', 'nord', 'emerald', 'autumn',
    'forest', 'winter', 'night', 'dim', 'coffee', 'sunset', 'luxury', 'black',
    'silk', 'lofi', 'wireframe'
];

const storedTheme = localStorage.getItem('aqiu-theme');
const selectedTheme = ref(themeOptions.includes(storedTheme || '') ? storedTheme! : 'aqiu');

const applyTheme = (theme: string) => {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('aqiu-theme', theme);
};

watch(selectedTheme, (theme) => applyTheme(theme));

export function useTheme() {
    return {
        themeOptions,
        selectedTheme,
        applyTheme
    };
}
