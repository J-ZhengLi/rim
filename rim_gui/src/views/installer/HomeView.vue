<script lang="ts" setup>
import { onMounted, ref, watch } from 'vue';
import { useCustomRouter } from '@/router/index';
import { installConf, invokeCommand } from '@/utils/index';
import { open } from '@tauri-apps/api/dialog';
import { useI18n } from 'vue-i18n';

const { routerPush } = useCustomRouter();
const { locale } = useI18n();

const labels = ref<Record<string, string>>({});
const showCustomizePanel = ref(false);

// “install other edition” options
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

async function refreshLabels() {
  labels.value.toolkitName = await invokeCommand('toolkit_name') as string;
  labels.value.content_source = await invokeCommand('get_build_cfg_locale_str', { key: 'content_source' }) as string;
}

onMounted(async () => await refreshLabels());
watch(locale, async (_newVal) => await refreshLabels());
</script>

<template>
  <div flex="~ col items-center" w="full">
    <base-card h="60%" w="80%" class="info-card">
      <div flex="~ col items-center" h="full">
        <div text="center" class="toolkit-info">
          <div c="darker-secondary" font="bold" text="4vh">{{ labels.toolkitName }}</div>
          <div c="secondary" text="3.5vh">{{ installConf.version }}</div>
        </div>
        <base-button theme="primary" w="20vw" position="fixed" bottom="10vh" @click="handleInstallClick()">{{
          $t('install') }}</base-button>
        <span c="secondary" position="fixed" bottom="-5vh" cursor-pointer underline @click="showCustomizePanel = true">
          {{ $t('install_using_toolkit_manifest') }}
        </span>
      </div>
    </base-card>

    <base-panel width="60%" :show="showCustomizePanel" @close="showCustomizePanel = false">
      <div flex="~ col">
        <b class="option-label">{{ $t('toolkit_manifest_path') }}</b>
        <inputton m="1rem" h="5vh" v-bind:modelValue="toolkitManifestPath" :button-label="$t('select_file')"
          @change="(event: Event) => toolkitManifestPath = (event.target as HTMLInputElement).value"
          @keydown.enter="(event: Event) => toolkitManifestPath = (event.target as HTMLInputElement).value"
          @button-click="pickToolkitSource" />
        <div flex="~ justify-center" mt="4vh">
          <base-button :disabled="!toolkitManifestPath" w="20vw" theme="primary" @click="confirmCustomizedEdition">{{
            $t('confirm') }}</base-button>
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
