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
                            <svg width="20px" height="20px" mr="1rem" py="2px" viewBox="0 0 24 24" fill="none"
                                xmlns="http://www.w3.org/2000/svg">
                                <path
                                    d="M2 12C2 17.5228 6.47715 22 12 22C17.5228 22 22 17.5228 22 12C22 6.47715 17.5228 2 12 2C6.47715 2 2 6.47715 2 12Z"
                                    stroke="#000000" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                                </path>
                                <path d="M13 2.04932C13 2.04932 16 5.99994 16 11.9999C16 17.9999 13 21.9506 13 21.9506"
                                    stroke="#000000" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                                </path>
                                <path d="M11 21.9506C11 21.9506 8 17.9999 8 11.9999C8 5.99994 11 2.04932 11 2.04932"
                                    stroke="#000000" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                                </path>
                                <path d="M2.62964 15.5H21.3704" stroke="#000000" stroke-width="1.5"
                                    stroke-linecap="round" stroke-linejoin="round"></path>
                                <path d="M2.62964 8.5H21.3704" stroke="#000000" stroke-width="1.5"
                                    stroke-linecap="round" stroke-linejoin="round"></path>
                            </svg>
                            <span class="label-text">{{ $t('language') }}</span>
                        </div>
                        <base-select width="12vw" :items="langOptions" v-model="language" />
                    </label>
                </div>
            </section>

            <section ref="updateRef" class="setting-section">
                <h2>{{ $t('update') }}</h2>
                <div class="setting-options">
                    <label>
                        <span class="label-text">{{ $t('manager_update_channel') }}</span>
                        <base-select width="12vw" :items="updateChannels" v-model="rimUpdateChannel" />
                    </label>
                    <base-check-box v-model="autoCheckRimUpdate" :title="$t('auto_check_manager_updates')"
                        labelAlignment="left" />
                    <base-check-box v-model="autoCheckToolkitUpdate" :title="$t('auto_check_toolkit_updates')"
                        labelAlignment="left" />
                    <label>
                        <span class="label-text">{{ $t('check_manager_updates') }}</span>
                        <base-button w="12vw" theme="primary" @click="checkManagerUpdates">
                            <spinner v-if="isChecking" />
                            <span v-else>{{ $t('check') }}</span>
                        </base-button>
                    </label>
                </div>
            </section>
        </main>
    </div>
</template>

<script setup lang="ts">
import { DropdownItem } from '@/components/BaseSelect.vue';
import { invokeCommand } from '@/utils';
import { ref, computed, watch, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';

const { locale, availableLocales, t } = useI18n();

const generalRef = ref<HTMLElement | null>(null);
const updateRef = ref<HTMLElement | null>(null);
const mainContentRef = ref<HTMLElement | null>(null);
const isChecking = ref(false);

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

function checkManagerUpdates() {
    isChecking.value = true;
    invokeCommand('check_manager_update').then(() => isChecking.value = false);
}

// RIM configuration but without `language` as it was already handled by `i18n.ts`
interface Configuration {
    update: {
        'auto-check-manager-updates': boolean,
        'auto-check-toolkit-updates': boolean,
        'manager-update-channel': 'stable' | 'beta',
    }
}

// setting options
const language = ref(locale.value);
const autoCheckRimUpdate = ref(true);
const autoCheckToolkitUpdate = ref(true);
const rimUpdateChannel = ref(updateChannels.value[0].value);

onMounted(async () => {
    // load RIM configuration
    const rimConf = await invokeCommand('get_rim_configuration') as Configuration;
    autoCheckRimUpdate.value = rimConf.update['auto-check-manager-updates'];
    autoCheckToolkitUpdate.value = rimConf.update['auto-check-toolkit-updates'];
    rimUpdateChannel.value = rimConf.update['manager-update-channel'];
});

watch(language, (current) => {
    locale.value = current;
    invokeCommand('set_locale', { language: current });
});
watch(autoCheckRimUpdate, (val) => {
    invokeCommand('set_auto_check_manager_updates', { yes: val });
});
watch(autoCheckToolkitUpdate, (val) => {
    invokeCommand('set_auto_check_toolkit_updates', { yes: val });
});
watch(rimUpdateChannel, (val) => {
    invokeCommand('set_manager_update_channel', { channel: val });
});
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
