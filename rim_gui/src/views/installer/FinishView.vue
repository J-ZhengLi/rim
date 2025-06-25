<script setup lang="ts">
import { onMounted, Ref, ref } from 'vue';
import { installConf, invokeCommand, invokeLabelList } from '@/utils/index';

const runApp = ref(true);
const createShortcut = ref(true);
const labels: Ref<Record<string, string>> = ref({});

async function closeWindow() {
  await invokeCommand('post_installation_opts', {
    installDir: installConf.path.value,
    open: runApp.value,
    shortcut: createShortcut.value
  });
}

onMounted(() => {
  invokeLabelList([
    'install_finish_info',
    'finish',
    'post_installation_open',
    'post_installation_create_shortcut',
  ]).then((res) => labels.value = res);
});
</script>

<template>
  <div flex="~ col">
    <div flex="1">
      <span class="info-label">{{ labels.install_finish_info }}</span>
      <div my="4vh" mx="2vw" flex="~ col" gap="4vh">
        <base-check-box v-model="runApp" :title="labels.post_installation_open"/>
        <base-check-box v-model="createShortcut" :title="labels.post_installation_create_shortcut" />
      </div>
    </div>
    <page-nav-buttons :hideBack="true" :nextLabel="labels.finish" @next-clicked="closeWindow" />
  </div>
</template>
