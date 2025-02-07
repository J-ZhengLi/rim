<script lang="ts" setup>
import { computed, onBeforeMount, onMounted, ref } from 'vue';
import { useCustomRouter } from '@/router/index';
import { message } from '@tauri-apps/api/dialog';
import { installConf, invokeCommand, invokeLabelList } from '@/utils/index';

const { routerPush } = useCustomRouter();
const isDialogVisible = ref(false);
// TODO: add license and app description etc
const explainText: string[] = ``.split('\n');

const isUserAgree = ref(true);
const welcomeLabel = ref('');
const labels = ref<Record<string, string>>({});
const version = computed(() => installConf.version);

function handleDialogOk() {
  isDialogVisible.value = false;
  isUserAgree.value = true;
}

function handleInstallClick(custom: boolean) {
  if (isUserAgree.value) {
    installConf.setCustomInstall(custom);
    routerPush(custom ? '/installer/folder' : '/installer/confirm');
  } else {
    message('请先同意许可协议', { title: '提示' });
  }
}

onBeforeMount(() => installConf.loadManifest());

invokeCommand('welcome_label').then((lb) => {
  if (typeof lb === 'string') {
    welcomeLabel.value = lb;
  }
});
onMounted(() => {
  const labelKeys = [
    'welcome',
    'vendor',
    'install',
    'source_hint',
    'license_agreement',
    'close',
    'agree',
  ];
  invokeLabelList(labelKeys).then((results) => {
    labels.value = results;
  });
});
</script>

<template>
  <div class="svg-background" flex="~ col items-center" w="full">
    <div grow="2">
      <a
        block
        mt="15vw"
        decoration="none"
        flex="~ items-center"
        href="https://xuanwu.beta.atomgit.com/"
        target="_blank"
      >
        <img class="logo" src="/logo.png" alt="logo" />
        <div ml="12px" c="header" font="bold" text="[clamp(24px,4vw,40px)]">
          {{ labels.vendor }}
        </div>
      </a>
    </div>
    <div grow="2" flex="~ col items-center">
      <div class="bold-text" text="[clamp(22px,3.6vw,38px)]">{{ welcomeLabel }}</div>
      <div class="bold-text" text="[clamp(12px,2vw,24px)]">{{ version }}</div>
    </div>
    <div w="full" text="center">
      <div flex="~ items-end justify-center">
        <base-button
          theme="primary"
          w="12rem"
          mx="8px"
          font="bold"
          @click="handleInstallClick(true)"
          >{{ labels.install }}</base-button
        >
      </div>
      <!-- <base-check-box v-model="isUserAgree" mt="8px"
        >我同意
        <span
          @click="isDialogVisible = !isDialogVisible"
          c="primary"
          cursor-pointer
          decoration="hover:underline"
          >许可协议</span
        >
      </base-check-box> -->
    </div>
    <div basis="30px" m="10px" text="center [clamp(11px,1vw,16px)]">
      {{ labels.source_hint }}
    </div>
    <base-dialog
      v-model="isDialogVisible"
      title="{{ labels.license_agreement }}"
      width="80%"
    >
      <scroll-box flex="1" overflow="auto">
        <p v-for="txt in explainText" :key="txt">
          {{ txt }}
        </p>
      </scroll-box>
      <template #footer>
        <div flex="~ items-center justify-end" gap="12px" mt="12px">
          <base-button @click="isDialogVisible = !isDialogVisible">{{
            labels.close
          }}</base-button>
          <base-button @click="handleDialogOk">{{ labels.agree }}</base-button>
        </div>
      </template>
    </base-dialog>
  </div>
</template>

<style lang="css" scoped>
.bold-text {
  text-align: center;
  line-height: 6dvw;
  cursor: default;
  font-weight: bold;
  margin-inline: 10px;
}
.svg-background {
  background-image: url("/installer_bg.svg");
  background-repeat: no-repeat;
  background-position:
    top center,
    bottom center;
  background-size: 100% auto;
}
.logo {
  height: clamp(45px, 10vw, 80px);
}
</style>
