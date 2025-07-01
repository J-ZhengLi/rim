<script lang="ts" setup>
import { onMounted, ref } from 'vue';
import { useCustomRouter } from '@/router/index';
import { installConf, invokeCommand, invokeLabelList } from '@/utils/index';
import { open } from '@tauri-apps/api/dialog';

const { routerPush } = useCustomRouter();

const toolkitName = ref('');
const labels = ref<Record<string, string>>({});
const showCustomizePanel = ref(false);

// “install other edtion” options
const toolkitManifestPath = ref('');

function handleInstallClick() {
  routerPush('/installer/configuration');
}

async function confirmCustomizedEdition() {
  await installConf.loadManifest(toolkitManifestPath.value);
  showCustomizePanel.value = false;
  console.log(installConf.version);
}

async function pickToolkitSource() {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{
      name: 'TOML File',
      extensions: ['toml']
    }],
  });
  if (selected && typeof selected === 'string') {
    toolkitManifestPath.value = selected;
  }
}

onMounted(() => {
  const labelKeys = [
    'install',
    'install_using_toolkit_manifest',
    'confirm',
    'native',
    'select_file',
  ];
  invokeLabelList(labelKeys).then((results) => {
    labels.value = results;
  });

  invokeCommand('toolkit_name').then((lb) => {
    if (typeof lb === 'string') {
      toolkitName.value = lb;
    }
  });

  invokeCommand('get_build_cfg_locale_str', { key: 'vendor' }).then((res) => {
    if (typeof res === 'string') {
      labels.value.vendor = res
    }
  });

  invokeCommand('get_build_cfg_locale_str', { key: 'content_source' }).then((res) => {
    if (typeof res === 'string') {
      labels.value.content_source = res
    }
  });
});
</script>

<template>
  <div flex="~ col items-center" w="full">
    <base-card h="60%" w="80%" class="info-card">
      <div flex="~ col items-center" h="full">
        <div text="center" class="toolkit-info">
          <div c="darker-secondary" font="bold" text="4vh">{{ toolkitName }}</div>
          <div c="secondary" text="3.5vh">{{ installConf.version }}</div>
        </div>
        <base-button theme="primary" w="20vw" position="fixed" bottom="10vh" @click="handleInstallClick()">{{
          labels.install }}</base-button>
        <span c="secondary" position="fixed" bottom="-5vh" cursor-pointer underline @click="showCustomizePanel = true">
          {{ labels.install_using_toolkit_manifest }}
        </span>
      </div>
    </base-card>

    <base-panel width="60%" :show="showCustomizePanel" @close="showCustomizePanel = false">
      <div flex="~ col">
        <b class="option-label">Toolkit Manifest Path</b>
        <inputton m="1rem" h="5vh" v-bind:modelValue="toolkitManifestPath" :button-label="labels.select_file"
          @change="(event: Event) => toolkitManifestPath = (event.target as HTMLInputElement).value"
          @keydown.enter="(event: Event) => toolkitManifestPath = (event.target as HTMLInputElement).value"
          @button-click="pickToolkitSource" />
        <div flex="~ justify-center" mt="4vh">
          <base-button :disabled="!toolkitManifestPath" w="20vw"
            theme="primary" @click="confirmCustomizedEdition">{{ labels.confirm }}</base-button>
        </div>
      </div>
    </base-panel>

    <div class="content-disclaimer">{{ labels.content_source }}</div>
  </div>
</template>

<style lang="css" scoped>
.toolkit-info {
  margin-top: 12vh;
  margin-bottom: 10vh;
  display: flex;
  flex-direction: column;
  gap: 5vh;
}

.info-card {
  top: 45%;
  position: absolute;
  transform: translateY(-50%);
}

.content-disclaimer {
  --uno: c-secondary;
  position: fixed;
  font-size: 14px;
  bottom: 3vh;
}

.option-label {
  --uno: 'c-regular';
  margin-bottom: 0.5rem;
  font-weight: 500;
  font-size: clamp(0.5rem, 2.6vh, 1.5rem);
  flex-shrink: 0;
}
</style>
