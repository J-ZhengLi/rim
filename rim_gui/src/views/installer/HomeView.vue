<script lang="ts" setup>
import { computed, onBeforeMount, onMounted, ref } from 'vue';
import { useCustomRouter } from '@/router/index';
import { installConf, invokeCommand, invokeLabelList } from '@/utils/index';

const { routerPush } = useCustomRouter();

const welcomeLabel = ref('');
const labels = ref<Record<string, string>>({});
const version = computed(() => installConf.version);

function handleInstallClick(custom: boolean) {
  installConf.setCustomInstall(custom);
  routerPush(custom ? '/installer/folder' : '/installer/confirm');
}

onBeforeMount(() => installConf.loadManifest());

onMounted(() => {
  const labelKeys = [
    'install',
  ];
  invokeLabelList(labelKeys).then((results) => {
    labels.value = results;
  });

  invokeCommand('welcome_label').then((lb) => {
    if (typeof lb === 'string') {
      welcomeLabel.value = lb;
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
  <div class="with-background" flex="~ col items-center" w="full">
    <div w="full" text="center">
      <div flex="~ items-end justify-center">
        <base-button theme="primary" w="12rem" mx="8px" font="bold" @click="handleInstallClick(true)">{{ labels.install
          }}</base-button>
      </div>
    </div>
    <div class="content-disclaimer">{{ labels.content_source }}</div>
  </div>
</template>

<style lang="css" scoped>
.with-background {
  background: linear-gradient(
    to bottom,
    white,
    white 25%,
    rgba(203, 223, 255, 0.87)
  );
  background-size: 100% auto;
  border: 1px solid rgb(166, 166, 166);
  box-sizing: border-box;
}

.content-disclaimer {
  color: rgb(128, 128, 128);
  position: fixed;
  font-size: 14px;
  bottom: 3vh;
}
</style>
