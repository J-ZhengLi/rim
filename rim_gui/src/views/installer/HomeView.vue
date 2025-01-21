<script lang="ts" setup>
import { computed, onBeforeMount, ref } from 'vue';
import { useCustomRouter } from '@/router/index';
import { message } from '@tauri-apps/api/dialog';
import { installConf, invokeCommand } from '@/utils/index';

const { routerPush } = useCustomRouter();
const isDialogVisible = ref(false);
// TODO: add license and app description etc
const explainText: string[] = ``.split(
  '\n'
);

const isUserAgree = ref(false);
const welcomeLabel = ref('');
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
})
</script>

<template>
  <div flex="~ col items-center" w="full">
    <div grow="2">
      <a href="https://xuanwu.beta.atomgit.com/" target="_blank">
        <img
          src="/logo.svg"
          alt="logo"
          w="200px"
          mt="50%"
        />
      </a>
    </div>
    <div grow="2" flex="~ col items-center">
      <h1>{{ welcomeLabel }}</h1>
      <h2>{{ version }}</h2>
    </div>
    <div basis="120px" w="full" text="center">
      <div flex="~ items-end justify-center">
        <base-button
          theme="primary"
          w="12rem"
          mx="8px"
          text="1.2rem"
          font="bold"
          @click="handleInstallClick(true)"
          >安装</base-button
        >
      </div>
      <!--base-check-box v-model="isUserAgree" mt="8px"
        >我同意
        <span
          @click="isDialogVisible = !isDialogVisible"
          c="primary"
          cursor-pointer
          decoration="hover:underline"
          >许可协议</span
        >
      </base-check-box -->
    </div>
    <base-dialog v-model="isDialogVisible" title="许可协议" width="80%">
      <scroll-box flex="1" overflow="auto">
        <p v-for="txt in explainText" :key="txt">
          {{ txt }}
        </p>
      </scroll-box>
      <template #footer>
        <div flex="~ items-center justify-end" gap="12px" mt="12px">
          <base-button @click="isDialogVisible = !isDialogVisible"
            >关闭</base-button
          >
          <base-button @click="handleDialogOk">我同意</base-button>
        </div>
      </template>
    </base-dialog>
  </div>
</template>
