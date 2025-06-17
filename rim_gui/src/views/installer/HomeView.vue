<script lang="ts" setup>
import { onBeforeMount, onMounted, ref } from 'vue';
import { useCustomRouter } from '@/router/index';
import { installConf, invokeCommand, invokeLabelList } from '@/utils/index';

const { routerPush } = useCustomRouter();

const toolkitName = ref('');
const labels = ref<Record<string, string>>({});

function handleInstallClick(custom: boolean) {
  installConf.setCustomInstall(custom);
  routerPush(custom ? '/installer/folder' : '/installer/confirm');
}

function selectOtherEdition() {
  console.log("TODO: show a page to load toolkit form url or path");
}

onBeforeMount(() => installConf.loadManifest());

onMounted(() => {
  const labelKeys = [
    'install',
    'install_other_edition',
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
    <base-card h="50%" w="60%" class="info-card">
      <div flex="~ col items-center" h="full">
        <div text="center" class="toolkit-info">
          <div c="darker-secondary" font="bold" text="4vh">{{ toolkitName }}</div>
          <div c="secondary" text="3.5vh">{{ installConf.version }}</div>
        </div>
        <base-button theme="primary" font="bold" w="20vw" position="fixed" bottom="4vh"
          @click="handleInstallClick(true)">{{ labels.install }}</base-button>
        <span c="secondary" position="fixed" bottom="-5vh" cursor-pointer underline @click="selectOtherEdition">{{
          labels.install_other_edition }}</span>
      </div>
    </base-card>
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
</style>
