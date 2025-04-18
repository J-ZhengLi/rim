<script setup lang="ts">
import { onMounted, Ref, ref } from 'vue';
import { Notification, NotificationAction, RustFunction } from './utils/types/Notification'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { invokeCommand, managerConf } from './utils';
import { message } from '@tauri-apps/plugin-dialog';
const appWindow = getCurrentWebviewWindow()

const appName = ref('');
const appIcon = "/128x128.png";
const notificationTitle = ref('');
const notificationContent = ref('');
const useNativeClose = ref(false);
const actions: Ref<NotificationAction[]> = ref([]);

function close() { appWindow.close() }

function onAction(command: RustFunction) {
  let func = command.name;
  let args = command.args;

  try {
    args ? invokeCommand(func, Object.fromEntries(args)) : invokeCommand(func);
  } catch (err) {
    if (err instanceof SyntaxError && args) {
      message("无效的 JSON 语法: " + args, { kind: 'error' });
    } else {
      message('调用 tauri 命令时发生错误: ' + err, { kind: 'error' });
    }
  }
}

onMounted(() => {
  managerConf.appName().then((res) => {
    appName.value = res;
  });

  invokeCommand('notification_content').then((res) => {
    let notification = res as Notification;
    notificationTitle.value = notification.title;
    notificationContent.value = notification.content;
    actions.value = notification.actions;
    useNativeClose.value = actions.value.length === 0;
  });
});
</script>

<template>
  <div class="notification-layout">
    <!-- notification contains two major areas,
    one for content (includes app name and icon), and one for actions -->
    <div class="header-box">
      <img :src="appIcon" class="app-icon" />
      <div class="title-box">
        <div class="app-name">{{ appName }}</div>
        <div class="notification-title">{{ notificationTitle }}</div>
      </div>
    </div>
    <div class="notification-content">{{ notificationContent }}</div>
    <div class="actions-box">
      <div class="action-layout" v-for="action in actions" :key="action.label" @click="onAction(action.command)">
        <img v-if="action.icon" :src="action.icon" class="action-icon" />
        <div class="action-label">{{ action.label }}</div>
      </div>
    </div>
  </div>
  <div class="close-button" @click="close" v-if="useNativeClose">
    <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 16 16">
      <path fill="white" fill-rule="evenodd"
        d="M4.28 3.22a.75.75 0 0 0-1.06 1.06L6.94 8l-3.72 3.72a.75.75 0 1 0 1.06 1.06L8 9.06l3.72 3.72a.75.75 0 1 0 1.06-1.06L9.06 8l3.72-3.72a.75.75 0 0 0-1.06-1.06L8 6.94z"
        clip-rule="evenodd" />
    </svg>
  </div>
</template>

<style scoped>
.notification-layout {
  width: 100%;
  height: 100%;
  left: 0;
  top: 0;
  position: absolute;
  text-align: center;
  background-color: rgb(38, 38, 38);
  overflow: hidden;
  /* Prevent body scrolling */
}

.header-box {
  display: flex;
  align-items: center;
}

.app-icon {
  position: relative;
  top: 0;
  left: 0;
  width: 48px;
  margin: 8px 2px 8px 8px;
}

.title-box {
  position: relative;
  right: 0;
  align-items: start;
  text-align: left;
  margin: 8px;
  overflow: hidden;
}

.app-name {
  display: -webkit-box;
  -webkit-box-orient: vertical;
  /* Limits to 2 lines */
  -webkit-line-clamp: 2;
  line-clamp: 2;
  overflow: hidden;
  text-overflow: ellipsis;
  word-break: break-word;
  white-space: normal;
  font-size: 1.1em;
  font-weight: bold;
  color: white;
}

.notification-title {
  font-size: 0.9em;
  color: rgb(191, 191, 191);
  margin-top: 5px;
}

.notification-content {
  text-align: left;
  color: rgb(191, 191, 191);
  position: absolute;
  left: 66px;
  right: 10px;
  height: 20vw;
  font-size: 0.9em;
  align-content: center;
  white-space: pre-wrap;
  /* Enable vertical scrolling */
  overflow-y: auto;
  scrollbar-width: none;
}

.close-button {
  display: flex;
  justify-content: center;
  align-items: center;
  width: 32px;
  height: 32px;
  position: absolute;
  top: 5px;
  right: 0px;
  padding: 5px;
}

.close-button:hover {
  background-color: rgb(50, 50, 50);
}

.actions-box {
  display: flex;
  position: absolute;
  bottom: 10px;
  left: 50%;
  transform: translateX(-50%);
  width: 80%;
  justify-content: space-around;
}

.action-layout {
  cursor: pointer;
}

.action-layout:hover .action-icon {
  background-color: rgb(50, 50, 50);
}

.action-icon {
  width: 6vw;
  padding: 10px;
}

.action-label {
  color: white;
  font-size: 0.8em;
  text-align: center;
  margin-top: -5px;
}
</style>
