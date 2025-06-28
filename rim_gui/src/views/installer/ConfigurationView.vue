<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useCustomRouter } from '@/router/index';
import { installConf, invokeCommand, invokeLabelList } from '@/utils/index';
import { message, open } from '@tauri-apps/api/dialog';

const { routerPush, routerBack } = useCustomRouter();
const labels = ref<Record<string, string>>({});

function handleNextClick() {
  // validate folder path input
  invokeCommand('check_install_path', { path: installConf.config.value.path }).then((res) => {
    if (typeof res === 'string') {
      message(res);
    } else {
      routerPush('/installer/customize_profile');
    }
  });
}

async function openFolder() {
  const selected = await open({
    multiple: false,
    directory: true,
    defaultPath: installConf.config.value.path,
  });
  if (selected && typeof selected === 'string') {
    installConf.setPath(selected.trim());
  }
}

onMounted(() => {
  invokeLabelList([
    'select_folder',
    'installation_path',
    'advanced_options',
    'system_configuration',
    'source_configuration',
    'add_to_path',
    'add_to_path_hint',
    'disable_ssl_cert_varification',
    'disable_ssl_cert_varification_hint',
    'rustup_dist_server',
    'rustup_dist_server_hint',
    'rustup_update_root',
    'rustup_update_root_hint',
    'cargo_registry_name',
    'cargo_registry_name_hint',
    'cargo_registry_index',
    'cargo_registry_index_hint',
  ]).then((res) => {
    labels.value = res;
  });
})
</script>

<template>
  <div flex="~ col">
    <div flex="1">
      <span class="info-label">{{ labels.installation_path }}</span>
      <inputton m="2vh" h="7vh" v-bind:modelValue="installConf.config.value.path" :button-label="labels.select_folder"
        @change="(event: Event) => installConf.setPath((event.target as HTMLInputElement).value)"
        @keydown.enter="(event: Event) => installConf.setPath((event.target as HTMLInputElement).value)"
        @button-click="openFolder" />
      <base-details my="4vh" mx="0.5vw" :title="labels.advanced_options">
        <base-card flex="~ col" m="1vh" overflow="auto" h="38vh">
          <b mb="0.5rem" text-regular>{{ labels.system_configuration }}</b>
          <base-check-box v-model="installConf.config.value.addToPath" :title="labels.add_to_path"
            :hint="labels.add_to_path_hint" />
          <base-check-box v-model="installConf.config.value.insecure" :title="labels.disable_ssl_cert_varification"
            :hint="labels.disable_ssl_cert_varification_hint" />
          <br></br>
          <b mb="0.5rem" text-regular>{{ labels.source_configuration }}</b>
          <labeled-input :disabled="!installConf.allowSourceConfig()"
            v-model="installConf.config.value.rustupDistServer" :label="labels.rustup_dist_server"
            :hint="labels.rustup_dist_server_hint" />
          <labeled-input :disabled="!installConf.allowSourceConfig()"
            v-model="installConf.config.value.rustupUpdateRoot" :label="labels.rustup_update_root"
            :hint="labels.rustup_update_root_hint" />
          <labeled-input v-model="installConf.config.value.cargoRegistryName" :label="labels.cargo_registry_name"
            :hint="labels.cargo_registry_name_hint" />
          <labeled-input :disabled="!installConf.allowSourceConfig()"
            v-model="installConf.config.value.cargoRegistryValue" :label="labels.cargo_registry_index"
            :hint="labels.cargo_registry_index_hint" />
        </base-card>
      </base-details>
    </div>
    <page-nav-buttons @back-clicked="routerBack" @next-clicked="handleNextClick" />
  </div>
</template>
