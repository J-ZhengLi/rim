<script setup lang="ts">
import { useCustomRouter } from '@/router';
import { invokeCommand, managerConf, Component, ComponentType, componentUtils } from '@/utils';
import { computed } from 'vue';
import ComponentLabel from './components/Label.vue';

const { routerPush, routerBack } = useCustomRouter();
const components = computed(() => managerConf.getTargetComponents());

const labels = computed(() => {
  const installed = managerConf.getInstalled();
  const installedToolchainVersion = installed?.components.find((c) => c.kind === ComponentType.ToolchainProfile)?.version;
  return components.value.map((item) => {
    const installedComponent = installed?.components.find((i) => i.name === item.name);
    let isFromToolchain = item.kind === ComponentType.ToolchainComponent || item.kind === ComponentType.ToolchainProfile;
    let installedVersion = isFromToolchain ? installedToolchainVersion : installedComponent?.version;
    return {
      label: item.displayName,
      originVer: installedVersion,
      targetVer: item.version,
    };
  });
});

const obsoletedComponents = computed(() => {
  const installedComponents = managerConf.getInstalled()?.components.map((comp) => comp.name);
  if (installedComponents) {
    return components.value.map((comp) => {
      return componentUtils(comp).obsoletes().filter((name) => installedComponents.includes(name));
    }).flat();
  }

  return [];
});

function handleNextClick() {
  invokeCommand('install_toolkit', {
    components_list: components.value as Component[],
  }).then(() => routerPush('/manager/progress'));
}
</script>

<template>
  <section flex="~ col" w="full" h="full">
    <div mx="12px">
      <h1>确认信息</h1>
      <p>即将安装以下产品</p>
    </div>

    <base-card mx="12px" flex="1">
      <div v-for="item in labels" :key="item.label" mb="24px">
        <component-label :label="item.label" :oldVer="item.originVer" :newVer="item.targetVer" />
      </div>
    </base-card>

    <div mx="12px" v-if="obsoletedComponents.length > 0">
      <p>{{ $t('components_to_remove') }}</p>
      <base-card flex="1">
        <div v-for="item in obsoletedComponents" mb="24px">
          <component-label :label="item" />
        </div>
      </base-card>
    </div>

    <div basis="60px" flex="~ justify-end items-center">
      <base-button theme="primary" mr="12px" @click="routerBack()">上一步</base-button>
      <base-button theme="primary" mr="12px" @click="handleNextClick">开始安装</base-button>
    </div>
  </section>
</template>
