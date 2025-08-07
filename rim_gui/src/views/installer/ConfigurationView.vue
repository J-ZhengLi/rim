<script setup lang="ts">
import { useCustomRouter } from '@/router/index';
import { installConf, invokeCommand } from '@/utils/index';
import { message, open } from '@tauri-apps/api/dialog';
import { computed } from 'vue';

const { routerPush, routerBack } = useCustomRouter();

// Enforceable Options
const rustupDistServer = computed({
  get: () => installConf.config.value.rustupDistServer?.[0] || '',
  set: (newValue: string) => {
    if (installConf.config.value.rustupDistServer) {
      installConf.config.value.rustupDistServer[0] = newValue;
    }
  }
});
const rustupUpdateRoot = computed({
  get: () => installConf.config.value.rustupUpdateRoot?.[0] || '',
  set: (newValue: string) => {
    if (installConf.config.value.rustupUpdateRoot) {
      installConf.config.value.rustupUpdateRoot[0] = newValue;
    }
  }
});
const cargoRegistryName = computed({
  get: () => installConf.config.value.cargoRegistryName?.[0] || '',
  set: (newValue: string) => {
    if (installConf.config.value.cargoRegistryName) {
      installConf.config.value.cargoRegistryName[0] = newValue;
    }
  }
});
const cargoRegistryValue = computed({
  get: () => installConf.config.value.cargoRegistryValue?.[0] || '',
  set: (newValue: string) => {
    if (installConf.config.value.cargoRegistryValue) {
      installConf.config.value.cargoRegistryValue[0] = newValue;
    }
  }
});

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
          <base-check-box v-model="installConf.config.value.insecure" :title="$t('disable_ssl_cert_verification')"
            :hint="$t('disable_ssl_cert_verification_hint')" />
          <br></br>
          <b mb="0.5rem" text-regular>{{ $t('source_configuration') }}</b>
          <labeled-input v-model="rustupDistServer" :label="$t('rustup_dist_server')"
            :disabled="installConf.config.value.rustupDistServer?.[1]" :disabledReason="$t('config_disabled_reason')"
            :hint="$t('rustup_dist_server_hint')" />
          <labeled-input v-model="rustupUpdateRoot" :label="$t('rustup_update_root')"
            :disabled="installConf.config.value.rustupUpdateRoot?.[1]" :disabledReason="$t('config_disabled_reason')"
            :hint="$t('rustup_update_root_hint')" />
          <labeled-input v-model="cargoRegistryName" :label="$t('cargo_registry_name')"
            :disabled="installConf.config.value.cargoRegistryName?.[1]" :disabledReason="$t('config_disabled_reason')"
            :hint="$t('cargo_registry_name_hint')" />
          <labeled-input v-model="cargoRegistryValue" :label="$t('cargo_registry_index')"
            :disabled="installConf.config.value.cargoRegistryValue?.[1]" :disabledReason="$t('config_disabled_reason')"
            :hint="$t('cargo_registry_index_hint')" />
        </base-card>
      </base-details>
    </div>
    <page-nav-buttons :backLabel="$t('back')" :nextLabel="$t('next')" @back-clicked="routerBack"
      @next-clicked="handleNextClick" />
  </div>
</template>
