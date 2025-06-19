<script setup lang="ts">
import { installConf, invokeCommand } from "@/utils";
import { appWindow } from "@tauri-apps/api/window";
import { onMounted, ref } from "vue";
import { event } from "@tauri-apps/api";

const { isSetupMode } = defineProps({
    isSetupMode: {
        type: Boolean,
        default: true,
    },
});

const exitDisabled = ref(false);
const labels = ref<Record<string, string>>({});
const appTitle = ref('');

function minimize() { appWindow.minimize(); }
function maximize() { appWindow.toggleMaximize() }
function close() {
    invokeCommand('close_window');
}

onMounted(() => {
    invokeCommand('get_build_cfg_locale_str', { key: 'logo_text' }).then((res) => {
        if (typeof res === 'string') {
            labels.value.logoText = res
        }
    });
    
    event.listen('toggle-exit-blocker', (event) => {
        if (typeof event.payload === 'boolean') {
            exitDisabled.value = event.payload;
        }
    });

    installConf.appNameWithVersion().then((res) => {
        appTitle.value = res
    });
})
</script>

<template>
    <div data-tauri-drag-region class="titlebar">
        <div class="titlebar-logo" id="titlebar-logo">
            <img data-tauri-drag-region src="/logo.png" h="7vh" />
            <div data-tauri-drag-region class="titlebar-logo-text">{{ labels.logoText }}</div>
        </div>
        <div data-tauri-drag-region class="titlebar-title" v-if="isSetupMode">{{ appTitle }}</div>

        <div data-tauri-drag-region class="titlebar-buttons" id="titlebar-buttons">
            <!-- FIXME: we need an English translation for GUI before enabling this -->
            <div class="titlebar-button">
                <svg xmlns="http://www.w3.org/2000/svg" width="18" viewBox="0 0 18 24">
                    <path d="M2 8C2 7.44772 2.44772 7 3 7H21C21.5523 7 22 7.44772 22 8C22 8.55228 21.5523 9 21 9H3C2.44772 9 2 8.55228 2 8Z"></path>
                    <path d="M2 12C2 11.4477 2.44772 11 3 11H21C21.5523 11 22 11.4477 22 12C22 12.5523 21.5523 13 21 13H3C2.44772 13 2 12.5523 2 12Z"></path>
                    <path d="M3 15C2.44772 15 2 15.4477 2 16C2 16.5523 2.44772 17 3 17H15C15.5523 17 16 16.5523 16 16C16 15.4477 15.5523 15 15 15H3Z"></path>
                </svg>
            </div>

            <div class="titlebar-button" id="titlebar-minimize" @click="minimize">
                <svg xmlns="http://www.w3.org/2000/svg" width="18" viewBox="0 0 16 16">
                    <path d="M3 8a.75.75 0 0 1 .75-.75h8.5a.75.75 0 0 1 0 1.5h-8.5A.75.75 0 0 1 3 8" />
                </svg>
            </div>

            <div class="titlebar-button" id="titlebar-maximize" @click="maximize">
                <svg xmlns="http://www.w3.org/2000/svg" width="18" viewBox="0 0 16 16">
                    <path d="M4.5 3A1.5 1.5 0 0 0 3 4.5v7A1.5 1.5 0 0 0 4.5 13h7a1.5 1.5 0 0 0 1.5-1.5v-7A1.5 1.5 0 0 0 11.5 3zM5 4.5h6a.5.5 0 0 1 .5.5v6a.5.5 0 0 1-.5.5H5a.5.5 0 0 1-.5-.5V5a.5.5 0 0 1 .5-.5" />
                </svg>
            </div>

            <div class="titlebar-button" id="titlebar-close" @click="close" v-if="!exitDisabled">
                <svg xmlns="http://www.w3.org/2000/svg" width="20" viewBox="0 0 16 16">
                    <path fill-rule="evenodd"
                        d="M4.28 3.22a.75.75 0 0 0-1.06 1.06L6.94 8l-3.72 3.72a.75.75 0 1 0 1.06 1.06L8 9.06l3.72 3.72a.75.75 0 1 0 1.06-1.06L9.06 8l3.72-3.72a.75.75 0 0 0-1.06-1.06L8 6.94z"
                        clip-rule="evenodd" />
                </svg>
            </div>
        </div>
    </div>
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
    margin-inline: 2.5vw;
}

.titlebar-logo {
    display: flex;
    align-items: center;
    margin-top: 2.5vh;
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
    margin-right: 0;
}

.titlebar-button {
    display: flex;
    justify-content: center;
    align-items: center;
    width: 4vw;
    height: 4vh;
    border-radius: 3px;
    margin-inline: 3px;
    padding: 0;
    fill:rgb(155, 155, 155);
}

.titlebar-button:hover {
    background: #696969;
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

.sub-menu {
    position: absolute;
    background-color: rgba(2, 2, 10, 0.8);
    transform: translateY(70%);
    border-radius: 3px;
}

.sub-menu ul {
    margin: 0;
    padding: 0;
}

.sub-menu ul li {
    list-style: none;
    display: flex;
    padding: 1rem;
    color: white;
    font-size: 14px;
    text-decoration: none;
}

.sub-menu ul li:hover {
  background-color: #526ecc;
}

.fade-leave-active {
    transition: all .5s ease-out;
}

.fade-leave-to {
    opacity: 0;
}
</style>
