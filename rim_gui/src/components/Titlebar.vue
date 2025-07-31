<script setup lang="ts">
import { installConf, invokeCommand } from "@/utils";
import { appWindow } from "@tauri-apps/api/window";
import { computed, onMounted, ref, shallowRef, watch } from "vue";
import { event } from "@tauri-apps/api";
import { useI18n } from "vue-i18n";
import SettingsLayout from "@/layouts/SettingsLayout.vue";
import AboutLayout from "@/layouts/AboutLayout.vue";
import HelpLayout from "@/layouts/HelpLayout.vue";

interface MenuItem {
    icon?: string,
    label: string
    action: () => void
}

defineProps({
    showTitle: {
        type: Boolean,
        default: true,
    },
    bottomBorder: {
        type: Boolean,
        default: false,
    },
});
const { t, locale } = useI18n();

const exitDisabled = ref(false);
const labels = ref<Record<string, string>>({});
const appTitle = ref('');

// Drop-down menu controls
const isMenuShown = ref(false);
const isPanelShown = ref(false);
const layoutToShow = shallowRef<any>(null);
const menuItems = computed<MenuItem[]>(() => [
    {
        icon: '/icons/settings.svg',
        label: t('settings'),
        action: () => showPanel('settings'),
    },
    {
        icon: '/icons/help.svg',
        label: t('help'),
        action: () => showPanel('help'),
    },
    {
        icon: '/icons/info.svg',
        label: t('about'),
        action: () => showPanel('about'),
    },
    {
        icon: '/icons/exit.svg',
        label: t('exit'),
        action: () => close(),
    }
]);

function minimize() { appWindow.minimize(); }
function maximize() { appWindow.toggleMaximize() }
function close() {
    invokeCommand('close_window');
}

function showPanel(name: string) {
    switch (name) {
        case 'settings':
            layoutToShow.value = SettingsLayout;
            break;
        case 'about':
            layoutToShow.value = AboutLayout;
            break;
        case 'help':
            layoutToShow.value = HelpLayout;
            break;
        default:
            layoutToShow.value = null;
            break;
    }
    isPanelShown.value = true;
}

async function refreshLabels() {
    labels.value.logoText = await invokeCommand('get_build_cfg_locale_str', { key: 'logo_text' }) as string;
}

onMounted(async () => {
    await refreshLabels();

    event.listen('toggle-exit-blocker', (event) => {
        if (typeof event.payload === 'boolean') {
            exitDisabled.value = event.payload;
        }
    });

    appTitle.value = await installConf.appNameWithShortVersion();
});

watch(locale, (_) => refreshLabels());
</script>

<template>
    <div data-tauri-drag-region class="titlebar" :class="{ 'titlebar-border': bottomBorder }">
        <div class="titlebar-logo" id="titlebar-logo">
            <img data-tauri-drag-region src="/logo.png" h="7vh" />
            <div data-tauri-drag-region class="titlebar-logo-text">{{ labels.logoText }}</div>
        </div>
        <div data-tauri-drag-region class="titlebar-title" v-if="showTitle">{{ appTitle }}</div>

        <slot name="nav"></slot>

        <div data-tauri-drag-region class="titlebar-buttons" id="titlebar-buttons">
            <!-- expandable menu button -->
            <div class="titlebar-button" @click="isMenuShown = !isMenuShown">
                <svg xmlns="http://www.w3.org/2000/svg" width="18" viewBox="0 0 18 24">
                    <path
                        d="M2 8C2 7.44772 2.44772 7 3 7H21C21.5523 7 22 7.44772 22 8C22 8.55228 21.5523 9 21 9H3C2.44772 9 2 8.55228 2 8Z">
                    </path>
                    <path
                        d="M2 12C2 11.4477 2.44772 11 3 11H21C21.5523 11 22 11.4477 22 12C22 12.5523 21.5523 13 21 13H3C2.44772 13 2 12.5523 2 12Z">
                    </path>
                    <path
                        d="M3 15C2.44772 15 2 15.4477 2 16C2 16.5523 2.44772 17 3 17H15C15.5523 17 16 16.5523 16 16C16 15.4477 15.5523 15 15 15H3Z">
                    </path>
                </svg>
                <div class="menu-wrapper">
                    <transition name="dropdown">
                        <ul v-if="isMenuShown" class="dropdown-menu">
                            <li v-for="(item, index) in menuItems" :key="index" class="menu-item" @click="item.action">
                                <img :src="item.icon" class="icon" />
                                <span class="label">{{ item.label }}</span>
                            </li>
                        </ul>
                    </transition>
                </div>
            </div>
            <!-- minimize button -->
            <div class="titlebar-button" id="titlebar-minimize" @click="minimize">
                <svg xmlns="http://www.w3.org/2000/svg" width="18" viewBox="0 0 16 16">
                    <path d="M3 8a.75.75 0 0 1 .75-.75h8.5a.75.75 0 0 1 0 1.5h-8.5A.75.75 0 0 1 3 8" />
                </svg>
            </div>
            <!-- maximize button -->
            <div class="titlebar-button" id="titlebar-maximize" @click="maximize">
                <svg xmlns="http://www.w3.org/2000/svg" width="18" viewBox="0 0 16 16">
                    <path
                        d="M4.5 3A1.5 1.5 0 0 0 3 4.5v7A1.5 1.5 0 0 0 4.5 13h7a1.5 1.5 0 0 0 1.5-1.5v-7A1.5 1.5 0 0 0 11.5 3zM5 4.5h6a.5.5 0 0 1 .5.5v6a.5.5 0 0 1-.5.5H5a.5.5 0 0 1-.5-.5V5a.5.5 0 0 1 .5-.5" />
                </svg>
            </div>
            <!-- close button -->
            <div class="titlebar-button" id="titlebar-close" @click="close" v-if="!exitDisabled">
                <svg xmlns="http://www.w3.org/2000/svg" width="20" viewBox="0 0 16 16">
                    <path fill-rule="evenodd"
                        d="M4.28 3.22a.75.75 0 0 0-1.06 1.06L6.94 8l-3.72 3.72a.75.75 0 1 0 1.06 1.06L8 9.06l3.72 3.72a.75.75 0 1 0 1.06-1.06L9.06 8l3.72-3.72a.75.75 0 0 0-1.06-1.06L8 6.94z"
                        clip-rule="evenodd" />
                </svg>
            </div>
        </div>
    </div>
    <base-panel :show="isPanelShown" @close="isPanelShown = false">
        <component :is="layoutToShow" />
    </base-panel>
</template>

<style scoped>
.titlebar {
    background-color: rgba(0, 0, 0, 0);
    height: 10vh;
    user-select: none;
    display: flex;
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 1000;
}

.titlebar-border {
    padding-bottom: 1.5vh;
    border-bottom: 1px solid #ddd;
}

.titlebar-logo {
    display: flex;
    align-items: center;
    margin-top: 2vh;
    margin-left: 2.5vw;
}

.titlebar-logo-text {
    margin-left: 10px;
    font-weight: bold;
    font-size: 2.5vw;
}

.titlebar-buttons {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    margin-left: auto;
    margin-top: 2vh;
    margin-right: 2.5vw;
}

.titlebar-button {
    display: flex;
    justify-content: center;
    align-items: center;
    width: 4vw;
    height: 4vh;
    border-radius: 5px;
    margin-inline: 3px;
    padding: 5px 2px;
    fill: rgb(155, 155, 155);
}

.titlebar-button:hover {
    background: rgb(145, 145, 145);
    fill: white;
}

#titlebar-close:hover {
    background-color: #ff1528;
}

.titlebar-title {
    --uno: 'c-secondary';
    display: flex;
    margin: 3.2% 0px 0px 12px;
    font-size: 2.3vh;
}

.menu-wrapper {
    position: relative;
    margin-top: 5vh;
}

.dropdown-menu {
    position: absolute;
    right: 0;
    padding: 0;
    border-radius: 20px;
    background: rgba(255, 255, 255, .5);
    border: 2px solid transparent;
    box-shadow: 0 0 0 2px rgba(255, 255, 255, .6), 0 16px 32px rgba(0, 0, 0, .12);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    overflow: hidden;
    list-style: none;
    transform-origin: top center;
}

.menu-item {
    min-width: 5vw;
    display: flex;
    align-items: center;
    padding: 2vh 4vw;
    cursor: pointer;
    transition: all 0.2s ease;
    gap: 2vw;
}

.menu-item:hover {
    --uno: 'bg-light-primary';
}

.icon {
    position: absolute;
    left: 15%;
    width: 1.5rem;
    height: 1.5rem;
}

.label {
    margin-left: 30%;
    font-weight: 500;
    font-size: clamp(0.5rem, 2.6vh, 1.5rem);
    --uno: 'text-regular';
}

/* Animation classes */
.dropdown-enter-active,
.dropdown-leave-active {
    transition: all 0.4s cubic-bezier(0.165, 0.84, 0.44, 1);
}

.dropdown-enter-from,
.dropdown-leave-to {
    opacity: 0;
    transform: scaleY(0.8) translateY(-10px);
}
</style>
