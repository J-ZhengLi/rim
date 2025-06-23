<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useCustomRouter } from '@/router/index';
import { installConf, invokeCommand, invokeLabelList } from '@/utils/index';
import { message, open } from '@tauri-apps/api/dialog';

const { routerPush, routerBack } = useCustomRouter();
const labels = ref<Record<string, string>>({});

function handleNextClick() {
  // validate folder path input
  invokeCommand('check_install_path', { path: installConf.path.value as string }).then((res) => {
    if (typeof res === 'string') {
      message(res);
    } else {
      routerPush('/installer/customize_profile');
    }
  });
}

async function openFolder() {
  const selected = await open({
    multiple: false,
    directory: true,
    defaultPath: installConf.path.value,
  });
  if (selected && typeof selected === 'string') {
    installConf.setPath(selected.trim());
  }
}

onMounted(() => {
  invokeLabelList(['select_folder', 'installation_path', 'advanced_options']).then((res) => {
    labels.value = res;
  });
})
</script>

<template>
  <div flex="~ col">
    <div flex="1" m="12px">
      <span class="info-label">{{ labels.installation_path }}</span>
      <inputton m="1vh" h="7vh" v-bind:modelValue="installConf.path.value" :button-label="labels.select_folder"
        @change="(event: Event) => installConf.setPath((event.target as HTMLInputElement).value)"
        @keydown.enter="(event: Event) => installConf.setPath((event.target as HTMLInputElement).value)"
        @button-click="openFolder" />
      <base-details :title="labels.advanced_options">
        <div>TODO</div>
      </base-details>
    </div>
    <page-nav-buttons @back-clicked="routerBack" @next-clicked="handleNextClick" />
  </div>
</template>

<style lang="css">
.info-label {
  --uno: "c-regular";
  font-weight: bold;
  font-size: clamp(8px, 2.6vh, 22px);
  margin-inline: 1vh;
}
</style>
