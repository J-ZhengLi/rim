<template>
    <div class="settings-layout">
        <!-- Side Panel -->
        <aside class="sidebar">
            <nav>
                <ul>
                    <li @click="scrollToSection('general')">{{ $t('general') }}</li>
                    <li @click="scrollToSection('update')">{{ $t('update') }}</li>
                </ul>
            </nav>
        </aside>

        <!-- Main Panel -->
        <main class="main-content" ref="mainContentRef">
            <section ref="generalRef" class="setting-section">
                <h2>{{ $t('general') }}</h2>
                <div class="setting-options">
                    <label>
                        <div flex="~ item-center" w="20vw">
                            <svg width="1.5rem" height="1.5rem" mr="1rem" py="2px" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                                <path d="M2 12C2 17.5228 6.47715 22 12 22C17.5228 22 22 17.5228 22 12C22 6.47715 17.5228 2 12 2C6.47715 2 2 6.47715 2 12Z" stroke="#000000" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"></path>
                                <path d="M13 2.04932C13 2.04932 16 5.99994 16 11.9999C16 17.9999 13 21.9506 13 21.9506" stroke="#000000" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"></path>
                                <path d="M11 21.9506C11 21.9506 8 17.9999 8 11.9999C8 5.99994 11 2.04932 11 2.04932" stroke="#000000" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"></path>
                                <path d="M2.62964 15.5H21.3704" stroke="#000000" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"></path>
                                <path d="M2.62964 8.5H21.3704" stroke="#000000" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"></path>
                            </svg>
                            <span class="label-text">{{ $t('language') }}</span>
                        </div>
                        <base-select :items="langOptions" v-model="settings.language" />
                    </label>
                </div>
            </section>

            <section ref="updateRef" class="setting-section">
                <h2>{{ $t('update') }}</h2>
                <div class="setting-options">
                    <label>
                        <span class="label-text">{{ $t('manager_update_channel') }}</span>
                        <base-select :items="updateChannels" v-model="settings.rimUpdateChannel" />
                    </label>
                    <base-check-box v-model="settings.autoCheckRimUpdate" :title="$t('auto_check_manager_updates')" labelAlignment="left" />
                    <base-check-box v-model="settings.autoCheckToolkitUpdate" :title="$t('auto_check_toolkit_updates')" labelAlignment="left" />
                </div>
            </section>
        </main>
    </div>
</template>

<script setup lang="ts">
import { DropdownItem } from '@/components/BaseSelect.vue';
import { invokeCommand } from '@/utils';
import { ref, reactive, watch, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';

const { locale, availableLocales, t } = useI18n();

const generalRef = ref<HTMLElement | null>(null);
const updateRef = ref<HTMLElement | null>(null);
const mainContentRef = ref<HTMLElement | null>(null);

const langOptions = computed<DropdownItem[]>(() => {
    return availableLocales.map((loc) => {
        return { value: loc, label: langName(loc) } as DropdownItem;
    });
});
const updateChannels = computed<DropdownItem[]>(() => {
    return [
        { value: 'stable', label: t('stable') },
        { value: 'beta', label: t('beta') },
    ];
});

const scrollToSection = (section: 'general' | 'update') => {
    const target = section === 'general' ? generalRef.value : updateRef.value
    if (target && mainContentRef.value) {
        const top = target.offsetTop
        mainContentRef.value.scrollTo({ top, behavior: 'smooth' })
    }
};

// Reactive settings model
const settings = reactive({
    language: 'en-US',
    autoCheckRimUpdate: true,
    autoCheckToolkitUpdate: true,
    rimUpdateChannel: 'stable',
});

function langName(lang: string): string {
    switch (lang) {
        case 'en-US':
            return 'English';
        case 'zh-CN':
            return '简体中文';
        default:
            return lang;
    }
}

onMounted(() => {
    settings.language = locale.value;
});

// Automatically apply settings on change
watch(settings, (newVal) => {
    // TODO: avoid re-config irrelevant settings.
    console.log(newVal);
    locale.value = newVal.language;
    invokeCommand('set_locale', { language: newVal.language });
}, { deep: true });
</script>

<style scoped>
.settings-layout {
    display: flex;
}

/* Sidebar */
.sidebar {
    width: 10rem;
    border-right: 1px solid #ccc;
    padding: 1rem;
    box-sizing: border-box;
}

.sidebar ul {
    list-style: none;
    padding: 0;
}

.sidebar li {
    padding: 0.5rem 0;
    cursor: pointer;
    color: #333;
}

.sidebar li:hover {
    text-decoration: underline;
}

/* Main Content */
.main-content {
    margin-top: 0px;
    min-width: 40vw;
    flex: 1;
    overflow-y: auto;
    padding: 1rem 2rem;
}

.setting-section {
    margin-bottom: 3rem;
}

.setting-section h2 {
    margin-bottom: 1rem;
    border-bottom: 1px solid #ddd;
    padding-bottom: 1rem;
    --uno: 'c-regular';
}

.setting-options {
    margin-inline: 1rem;
}

label {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin: 0.75rem 0;
    gap: 8vw;
}

.label-text {
    --uno: 'c-regular';
    font-weight: 500;
    font-size: clamp(0.5rem, 2.6vh, 1.5rem);
    flex-shrink: 0;
}
</style>
