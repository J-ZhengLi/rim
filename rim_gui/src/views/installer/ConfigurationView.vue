<script setup lang="ts">
import { useCustomRouter } from '@/router/index';
import { installConf, invokeCommand } from '@/utils/index';
import { message, open } from '@tauri-apps/api/dialog';

const { routerPush, routerBack } = useCustomRouter();

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
</script>

<template>
  <div flex="~ col">
    <div flex="1">
      <span class="info-label">{{ $t('installation_path') }}</span>
      <inputton m="2vh" h="7vh" v-bind:modelValue="installConf.config.value.path" :button-label="$t('select_folder')"
        @change="(event: Event) => installConf.setPath((event.target as HTMLInputElement).value)"
        @keydown.enter="(event: Event) => installConf.setPath((event.target as HTMLInputElement).value)"
        @button-click="openFolder" />
      <base-details my="4vh" mx="0.5vw" :title="$t('advanced_options')">
        <base-card flex="~ col" m="1vh" overflow="auto" h="38vh">
          <b mb="0.5rem" text-regular>{{ $t('system_configuration') }}</b>
          <base-check-box v-model="installConf.config.value.addToPath" :title="$t('add_to_path')"
            :hint="$t('add_to_path_hint')" />
          <base-check-box v-model="installConf.config.value.insecure" :title="$t('disable_ssl_cert_varification')"
            :hint="$t('disable_ssl_cert_varification_hint')" />
          <br></br>
          <b mb="0.5rem" text-regular>{{ $t('source_configuration') }}</b>
          <labeled-input :disabled="!installConf.allowSourceConfig()"
            v-model="installConf.config.value.rustupDistServer" :label="$t('rustup_dist_server')"
            :hint="$t('rustup_dist_server_hint')" />
          <labeled-input :disabled="!installConf.allowSourceConfig()"
            v-model="installConf.config.value.rustupUpdateRoot" :label="$t('rustup_update_root')"
            :hint="$t('rustup_update_root_hint')" />
          <labeled-input v-model="installConf.config.value.cargoRegistryName" :label="$t('cargo_registry_name')"
            :hint="$t('cargo_registry_name_hint')" />
          <labeled-input :disabled="!installConf.allowSourceConfig()"
            v-model="installConf.config.value.cargoRegistryValue" :label="$t('cargo_registry_index')"
            :hint="$t('cargo_registry_index_hint')" />
        </base-card>
      </base-details>
    </div>
    <page-nav-buttons @back-clicked="routerBack" @next-clicked="handleNextClick" />
  </div>
</template>
