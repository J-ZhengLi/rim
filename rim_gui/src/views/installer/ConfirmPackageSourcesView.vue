<script setup lang="ts">
import { useCustomRouter } from '@/router';
import { Component, installConf, invokeCommand, invokeLabelList, RestrictedComponent, toChecked } from '@/utils';
import { onMounted, Ref, ref } from 'vue';

const { routerPush, routerBack } = useCustomRouter();
const labels = ref<Record<string, string>>({});
const fields: Ref<RestrictedComponent[]> = ref([]);

function handleOpen() {
}

function handleNextClick() {
  invokeCommand('updated_package_sources', { raw: fields.value, selected: installConf.getCheckedComponents() }).then((res) => {
    const components = res as Component[];
    installConf.setComponents(toChecked(components));
  });
  routerPush('/installer/confirm');
}

onMounted(() => {
  fields.value = installConf.getRestrictedComponents();

  const labelKeys = [
    'select_file',
    'provide_package_source',
    'package_source_missing_info',
  ];
  invokeLabelList(labelKeys).then((results) => {
    labels.value = results;
  });
});
</script>

<template>
  <div flex="~ col">
    <div ml="12px">
      <h4 mb="4px">{{ labels.provide_package_source }}</h4>
      <p mt="4px">{{ labels.package_source_missing_info }}</p>
    </div>
    <scroll-box flex="1" mx="12px" overflow="auto">
      <div class="input-field" v-for="(field, index) in fields" :key="index">
        <p>{{ field.label }}</p>
        <div flex="~ items-center">
          <base-input v-bind:value="field.source" flex="1" type="text" placeholder="输入路径或 URL" @change="
            (event: Event) =>
              field.source = (event.target as HTMLInputElement).value
          " @keydown.enter="
            (event: Event) =>
              field.source = (event.target as HTMLInputElement).value
          " />
          <base-button theme="primary" ml="12px" @click="handleOpen">{{ labels.select_file }}</base-button>
        </div>
      </div>
    </scroll-box>
    <div h="60px" flex="~ justify-end items-center">
      <base-button theme="primary" mr="12px" @click="routerBack">上一步</base-button>
      <base-button theme="primary" mr="12px" @click="handleNextClick">下一步</base-button>
    </div>
  </div>
</template>
