<script setup lang="ts">
import { installConf, invokeCommand, Component } from '@/utils/index';
import { useCustomRouter } from '@/router/index';
import { computed } from 'vue';

const { routerPush, routerBack } = useCustomRouter();

const components = computed(() => {
  const list = installConf.getCheckedComponents();
  list.sort((a, b) => a.id - b.id);
  return list;
});

async function handleNextClick() {
  routerPush('/installer/install');
  await invokeCommand('install_toolchain', {
    componentsList: components.value as Component[],
    config: installConf.config.value,
  });
}
</script>

<template>
  <div flex="~ col">
    <div>
      <span class="info-label">{{ $t('review_configuration') }}</span>
      <p class="sub-info-label">{{ $t('review_installation_hint') }}</p>
    </div>
    <base-card flex="1" mx="1vw" mb="7%" overflow="auto">
      <p m="0" font="bold">{{ $t('installation_path') }}:</p>
      <p my="0.5rem" ml="2rem">{{ installConf.config.value.path }}</p>
      <p m="0" font="bold">{{ $t('components') }}:</p>
      <div ml="2rem">
        <p my="0.5rem" v-for="component in components" :key="component.displayName">
          {{
            `${component.displayName} ${component.installed ? '(installed, re-installing)' : component.required ? '(required)' : ''} `
          }}
        </p>
      </div>
    </base-card>

    <page-nav-buttons
      :nextLabel="$t('install')"
      @back-clicked="routerBack"
      @next-clicked="handleNextClick"
    />
  </div>
</template>
