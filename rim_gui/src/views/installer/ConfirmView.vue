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

async function handleNextClick() {
  routerPush('/installer/install');
  await invokeCommand('install_toolchain', {
    components_list: components.value as Component[],
    install_dir: path.value as string,
  });
}

onMounted(() => {
  invokeLabelList([
    'install',
    'review_configuration',
    'review_installation_hint',
    'installation_path',
    'components',
  ]).then((res) => {
    labels.value = res;
  })
})
</script>

<template>
  <div flex="~ col">
    <div>
      <span class="info-label">{{ labels.review_configuration }}</span>
      <p ml="1vw">{{ labels.review_installation_hint }}</p>
    </div>
    <base-card flex="1" mx="1vw" mb="7%" overflow="auto">
      <p m="0" font="bold">{{ labels.installation_path }}:</p>
      <p my="0.5rem" ml="2rem">{{ path }}</p>
      <p m="0" font="bold">{{ labels.components }}:</p>
      <div ml="2rem">
        <p my="0.5rem" v-for="component in components" :key="component.displayName">
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
