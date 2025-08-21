<script setup lang="ts">
import { computed, onBeforeMount, onMounted, ref, watch } from 'vue';
import { getAppNameWithVersion, managerConf } from '@/utils';
import { useCustomRouter } from '@/router';
import { useI18n } from 'vue-i18n';
import { event } from '@tauri-apps/api';
import { UpdatePayload } from '@/utils/types/payloads';
import ManagerUpdate from '@/layouts/ManagerUpdate.vue';

const footerText = ref('');
const { locale } = useI18n();
const currentManagerRelease = ref('');
const latestManagerRelease = ref('');
const showUpdatePrompt = ref(false);

const { isBack } = useCustomRouter();
const transitionName = computed(() => {
  if (isBack.value === true) return 'back';
  if (isBack.value === false) return 'push';
  return '';
});

async function refreshLabels() {
  footerText.value = (await getAppNameWithVersion()).join(' ');
}

onBeforeMount(async () => {
  await managerConf.load();
});

onMounted(async () => {
  await refreshLabels();

  event.listen('manager:update-available', (event) => {
    let payload = event.payload as UpdatePayload[];
    console.log('new RIM update detected:', payload[1]);

    currentManagerRelease.value = payload[0].version;
    latestManagerRelease.value = payload[1].version;
    showUpdatePrompt.value = true;
  });
});

watch(locale, async (_newVal) => await refreshLabels());
</script>

<template>
  <main p="4vh" flex="1" overflow="hidden" absolute top="0" right="0" left="0" bottom="0" class="main">
    <div h-full relative>
      <router-view v-slot="{ Component }">
        <transition :name="transitionName">
          <keep-alive>
            <component :is="Component" absolute w="full" h="full" />
          </keep-alive>
        </transition>
      </router-view>
    </div>
    <div flex="~ justify-center" class="footer-label">{{ footerText }}</div>
  </main>
  <base-panel :show="showUpdatePrompt" :clickToHide="false" width="50%">
    <ManagerUpdate :curVer="currentManagerRelease" :newVer="latestManagerRelease" @close="showUpdatePrompt = false" />
  </base-panel>
</template>

<style lang="css" scoped>
.main {
  box-sizing: border-box;
}
</style>
