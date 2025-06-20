<script setup lang="ts">
import { installConf, invokeCommand, Component, invokeLabelList } from '@/utils/index';
import { useCustomRouter } from '@/router/index';
import { computed, onMounted, ref } from 'vue';

const { routerPush, routerBack } = useCustomRouter();
const path = installConf.path;
const labels = ref<Record<string, string>>({});

const components = computed(() => {
  const list = installConf.getCheckedComponents();
  list.sort((a, b) => a.id - b.id);
  return list;
});

function handleNextClick() {
  invokeCommand('install_toolchain', {
    components_list: components.value as Component[],
    install_dir: path.value as string,
  }).then(() => routerPush('/installer/install'));
}

onMounted(() => {
  invokeLabelList(['install']).then((res) => {
    labels.value = res;
  })
})
</script>

<template>
  <div flex="~ col">
    <div ml="12px">
      <p mt="4px">开始安装之前，请确认安装信息无误。</p>
      <p mb="4px">单击“安装”以继续。如果需要修改配置请点击“上一步”。</p>
    </div>
    <base-card flex="1" mx="12px" mb="7%" overflow="auto">
      <p m="0">安装位置：</p>
      <p my="4px">{{ path }}</p>
      <p mb="8px">组件：</p>
      <div ml="12px">
        <p my="4px" v-for="component in components" :key="component.displayName">
          {{
            `${component.displayName} ${component.installed ? '(installed, re-installing)' : component.required ? '(required)' : ''} `
          }}
        </p>
      </div>
    </base-card>

    <page-nav-buttons
      :nextLabel="labels.install"
      @back-clicked="routerBack"
      @next-clicked="handleNextClick"
    />
  </div>
</template>
