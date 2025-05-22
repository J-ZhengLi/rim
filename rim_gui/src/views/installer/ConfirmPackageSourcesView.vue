<script setup lang="ts">
import { useCustomRouter } from '@/router';
import { CheckItem, Component, installConf, invokeCommand, invokeLabelList, RestrictedComponent } from '@/utils';
import { open } from '@tauri-apps/api/dialog';
import { computed, onMounted, Ref, ref } from 'vue';

const { routerPush, routerBack } = useCustomRouter();
const labels = ref<Record<string, string>>({});
const fields: Ref<RestrictedComponent[]> = ref([]);
const allSourcesAreFilled = computed(() => fields.value.every((field) => {
  const value = field.source || field.default;
  return value && value.trim() !== '';
}));

function handleOpen(index: number) {
  open({
    directory: false,
    multiple: false,
  }).then(res => {
    if (typeof res === 'string') {
      fields.value[index].source = res;
    }
  })
}

function handleNextClick() {
  const restrictedComps = fields.value.map((c) => {
    // use default value if user has left it empty.
    // Note that the `c.default` will never be undefined at this point,
    // otherwise this button would not be enabled.
    return {
      ...c,
      source: c.source || c.default,
    } as RestrictedComponent;
  });

  invokeCommand('updated_package_sources', { raw: restrictedComps, selected: installConf.getCheckedComponents() }).then((res) => {
    const updatedCompsRaw = res as Component[];
    const updated = installConf.checkComponents.value.map(origComp => {
      const newComp = updatedCompsRaw.find(updatedItem => origComp.value.name === updatedItem.name);
      if (newComp) {
        return {
          ...origComp,
          value: newComp,
        } as CheckItem<Component>;
      }
      return origComp;
    })
    installConf.setComponents(updated);
  });
  routerPush('/installer/confirm');
}

onMounted(() => {
  fields.value = installConf.getRestrictedComponents();

  const labelKeys = [
    'select_file',
    'default_source_hint',
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
      <p mt="4px" class="text-secondary">{{ labels.default_source_hint }}</p>
    </div>
    <scroll-box flex="1" mx="12px" overflow="auto">
      <div class="input-field" v-for="(field, index) in fields" :key="index">
        <p>{{ field.label }}</p>
        <div flex="~ items-center">
          <base-input v-bind:value="field.source" flex="1" type="text"
            :placeholder="field.default ? field.default : '输入路径或 URL'" @change="
              (event: Event) =>
                field.source = (event.target as HTMLInputElement).value
            " @keydown.enter="
            (event: Event) =>
              field.source = (event.target as HTMLInputElement).value
          " />
          <base-button theme="primary" ml="12px" @click="handleOpen(index)">{{ labels.select_file }}</base-button>
        </div>
      </div>
    </scroll-box>
    <div h="60px" flex="~ justify-end items-center">
      <base-button theme="primary" mr="12px" @click="routerBack">上一步</base-button>
      <base-button theme="primary" mr="12px" :disabled="!allSourcesAreFilled" @click="handleNextClick">下一步</base-button>
    </div>
  </div>
</template>
