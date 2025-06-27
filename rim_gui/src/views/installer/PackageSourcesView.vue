<script setup lang="ts">
import { useCustomRouter } from '@/router';
import { CheckItem, Component, installConf, invokeCommand, invokeLabelList, RestrictedComponent } from '@/utils';
import { open } from '@tauri-apps/api/dialog';
import { onMounted, Ref, ref, watch } from 'vue';

const { routerPush, routerBack } = useCustomRouter();
const labels = ref<Record<string, string>>({});
const fields: Ref<RestrictedComponent[]> = ref([]);
const allSourcesAreFilled = ref(false);

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
  routerPush('/installer/confirmation');
}

onMounted(() => {
  fields.value = installConf.getRestrictedComponents();

  const labelKeys = [
    'select_file',
    'enter_path_or_url',
    'default_source_hint',
    'provide_package_source',
    'package_source_missing_info',
  ];
  invokeLabelList(labelKeys).then((results) => {
    labels.value = results;
  });
});

watch(fields, (newVal) => {
  allSourcesAreFilled.value = newVal.every((field) => {
    const value = field.source || field.default;
    return value && value.trim() !== '';
  });
});
</script>

<template>
  <div flex="~ col">
    <span class="info-label">{{ labels.provide_package_source }}</span>
    <p class="sub-info-label">{{ labels.package_source_missing_info }}<br></br>{{ labels.default_source_hint }}</p>
    <base-card flex="1" mx="1rem" mt="1vh" mb="10vh" overflow="auto">
      <div v-for="(field, index) in fields" :key="index">
        <b text-regular>{{ field.label }}</b>
        <inputton mt="1rem" h="6vh" v-bind:modelValue="field.source" :button-label="labels.select_file"
          :placeholder="field.default ? field.default : labels.enter_path_or_url"
          @change="(event: Event) => field.source = (event.target as HTMLInputElement).value"
          @keydown.enter="(event: Event) => field.source = (event.target as HTMLInputElement).value"
          @button-click="handleOpen(index)"
        />
          <!-- <base-input v-bind:value="field.source" flex="1" type="text"
            :placeholder="field.default ? field.default : '输入路径或 URL'" @change="
              (event: Event) =>
                field.source = (event.target as HTMLInputElement).value
            " @keydown.enter="
            (event: Event) =>
              field.source = (event.target as HTMLInputElement).value
          " />
          <base-button theme="primary" ml="12px" @click="handleOpen(index)">{{ labels.select_file }}</base-button> -->
      </div>
    </base-card>
    <page-nav-buttons :hideNext="!allSourcesAreFilled" @back-clicked="routerBack" @next-clicked="handleNextClick" />
  </div>
</template>
