<script setup lang="ts">
import { onMounted, Ref, ref } from 'vue';
import { installConf, invokeCommand, invokeLabelList } from '@/utils/index';

const runApp = ref(true);
const createShortcut = ref(true);
const labels: Ref<Record<string, string>> = ref({});

async function closeWindow() {
  await invokeCommand('post_installation_opts', {
    installDir: installConf.config.value.path,
    open: runApp.value,
    shortcut: createShortcut.value
  });
}

onMounted(() => {
  invokeLabelList([
    'install_finish_info',
    'finish',
    'post_installation_hint',
    'post_installation_open',
    'post_installation_create_shortcut',
  ]).then((res) => labels.value = res);
});
</script>

<template>
  <div flex="~ col items-center">
    <base-card class="info-card">
      <div flex="~ col items-center" h="full">
        <div text="center" class="finish-info">
          <div c="darker-secondary" font="bold" text="4vh">{{ labels.install_finish_info  }}</div>
          <div c="secondary" text="3vh">{{ labels.post_installation_hint }}</div>
        </div>
        <div flex="~ col" gap="4vh">
          <base-check-box v-model="runApp" :title="labels.post_installation_open"/>
          <base-check-box v-model="createShortcut" :title="labels.post_installation_create_shortcut" />
        </div>
        <base-button theme="primary" w="20vw" position="fixed" bottom="5vh"
          @click="closeWindow()">{{ labels.finish }}</base-button>
      </div>
    </base-card>
  </div>
</template>

<style lang="css" scoped>
.finish-info {
  margin-top: 5vh;
  margin-bottom: 10vh;
  display: flex;
  flex-direction: column;
  gap: 3vh;
}

.info-card {
  position: absolute;
  left: 10%;
  right: 10%;
  top: 10%;
  bottom: 10%;
}
</style>
