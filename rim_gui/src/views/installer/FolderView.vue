<script setup lang="ts">
import { ref } from 'vue';
import { useCustomRouter } from '@/router/index';
import { installConf, invokeCommand } from '@/utils/index';
import { open } from '@tauri-apps/api/dialog';

const { routerPush, routerBack } = useCustomRouter();
// const diskRequire = ref(33);
const invalidReason = ref('');

function handleNextClick() {
  // validate folder path input
  invokeCommand('check_install_path', { path: installConf.path.value as string }).then((res) => {
    if (typeof res === 'string') {
      invalidReason.value = res
    } else {
      routerPush('/installer/components');
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
</script>

<template>
  <div flex="~ col">
    <div flex="1" mx="12px">
      <h4>安装目录</h4>
      <p>Rust 发行版将会安装到该路径中。</p>
      <div flex="~ items-center">
        <base-input v-bind:value="installConf.path.value" flex="1" type="text" placeholder="选择一个文件夹" @change="
          (event: Event) =>
            installConf.setPath((event.target as HTMLInputElement).value)
        " @keydown.enter="
            (event: Event) =>
              installConf.setPath((event.target as HTMLInputElement).value)
          " />
        <base-button theme="primary" ml="12px" @click="openFolder">选择文件夹</base-button>
      </div>
      <div flex="~ items-center">
        <p style="color:red">{{ invalidReason }}</p>
      </div>
    </div>
    <!-- <div mx="12px">
      <p>至少需要{{ diskRequire.toFixed(1) }}M的磁盘空间</p>
    </div> -->
    <div h="60px" flex="~ justify-end items-center">
      <base-button theme="primary" mr="12px" @click="routerBack">上一步</base-button>
      <base-button theme="primary" mr="12px" @click="handleNextClick">下一步</base-button>
    </div>
  </div>
</template>
