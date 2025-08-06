<script setup lang="ts">
import { ref, onMounted, nextTick, onBeforeUnmount, computed } from 'vue'
import Titlebar from './components/Titlebar.vue';
import { invokeCommand } from './utils';
import { event } from '@tauri-apps/api';
import { AppInfo } from './utils/types/AppInfo';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const managerMode = ref(false);
const navItems = computed(() => [
  {
    name: t('manage_toolkit'),
    showDot: ref(false),
  },
  {
    name: t('manage_components'),
    showDot: ref(false),
  },
  {
    name: t!('misc'),
    showDot: ref(false),
  },
]);
const selectedIndex = ref(0)
const navRefs = ref<HTMLElement[]>([])

// Store underline position and width
const underlineStyle = ref({
  left: '0px',
  width: '0px',
})

async function updateUnderline() {
  await nextTick();
  const el = navRefs.value[selectedIndex.value];
  if (el) {
    underlineStyle.value = {
      left: `${el.offsetLeft}px`,
      width: `${el.offsetWidth}px`,
    }
  }
}

async function selectNav(index: number) {
  selectedIndex.value = index;
  await updateUnderline();
}

async function updateDots() {
  if (!managerMode) return;

  event.listen('toolkit:update-available', (event) => {
    console.log("toolkit update available: ", event.payload);
    navItems.value[0].showDot.value = true;
  });
}

onMounted(async () => {
  const appInfo = await invokeCommand('app_info') as AppInfo;
  managerMode.value = appInfo.is_manager;

  updateUnderline();
  updateDots();

  window.addEventListener('resize', () => { updateUnderline() });
});

onBeforeUnmount(() => {
  window.removeEventListener('resize', () => { updateUnderline() });
})
</script>

<template>
  <background />
  <Titlebar :showTitle="!managerMode" :bottomBorder="managerMode">
    <template #nav v-if="managerMode">
      <nav class="nav-container">
        <ul class="nav-list">
          <li v-for="(item, index) in navItems" :key="index" class="nav-item"
            :class="{ active: selectedIndex === index }" @click="selectNav(index)" ref="navRefs">
            {{ item.name }}
            <span class="red-dot" v-if="item.showDot.value"></span>
          </li>
        </ul>
        <div class="underline" :style="underlineStyle"></div>
      </nav>
    </template>
  </Titlebar>
  <main>
    <router-view />
  </main>
</template>

<style>
:root {
  margin: 0;
  padding: 0;
  font-family:
    "Microsoft YaHei",
    "微软雅黑",
    Segoe UI,
    sans-serif;
  font-size: 14px;
  line-height: 24px;
  font-weight: 400;
  --uno: bg-back c-header;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

div {
  cursor: default;
}

main {
  margin-top: 10vh;
  overflow: hidden;
}

/* global scrollbar styling */
::-webkit-scrollbar {
  width: 5px;
  height: 5px;
}
::-webkit-scrollbar-track {
  background: transparent;
}
::-webkit-scrollbar-thumb {
  background-color: rgba(0, 0, 0, 0.2);
  border-radius: 4px;
  transition: background-color 0.3s ease;
}
:hover::-webkit-scrollbar-thumb {
  background-color: rgba(0, 0, 0, 0.4);
}

/* global toolkit styling */
.tooltip {
    position: absolute;
    bottom: calc(100% + 10px);
    left: 0;
    background-color: #333;
    color: white;
    padding: 0.5rem 1rem;
    border-radius: 4px;
    font-size: 12px;
    white-space: nowrap;
    z-index: 999;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
    pointer-events: none;
    opacity: 0.85;
}
.tooltip::after {
    content: '';
    position: absolute;
    top: 100%;
    left: 12px;
    border-width: 6px;
    border-style: solid;
    border-color: #333 transparent transparent transparent;
}

.info-label {
  --uno: "c-regular";
  font-weight: bold;
  font-size: clamp(8px, 2.6vh, 22px);
  margin-inline: 1vw;
}

.sub-info-label {
  --uno: "c-secondary";
  font-size: clamp(6px, 2.2vh, 20px);
  margin-inline: 1vw;
}

.footer-label {
  --uno: c-secondary;
  position: fixed;
  font-size: 14px;
  text-align: center;
  width: 100%;
  bottom: 3vh;
}
</style>

<style scoped>
.nav-container {
  position: relative;
  top: 4.5vh;
  width: 58%;
  height: 70%;
}

.nav-list {
  height: 100%;
  display: flex;
  justify-content: space-evenly;
  list-style: none;
  padding: 0;
  margin: 0;
}

.nav-item {
  position: relative;
  text-align: center;
  white-space: pre-line;
  --uno: "c-disabled";
  cursor: pointer;
  font-weight: 500;
  font-size: 2.6vh;
  transition: color 0.3s ease;
  padding-inline: 1rem;
}

.nav-item.active {
  --uno: "c-header";
}

.underline {
  position: absolute;
  bottom: 0;
  height: 3px;
  --uno: "bg-primary";
  transition: left 0.3s ease, width 0.3s ease;
}

.red-dot {
  position: absolute;
  top: 0;
  right: 0;
  width: 8px;
  height: 8px;
  background-color: red;
  border-radius: 50%;
}
</style>
