<script setup lang="ts">
import { useCustomRouter } from '@/router';
import { managerConf, ComponentType, componentUtils } from '@/utils';
import { computed, ref } from 'vue';
import ComponentLabel from './components/Label.vue';

const { routerPush, routerBack } = useCustomRouter();
const components = computed(() => managerConf.getTargetComponents());
const showConfigModifyPanel = ref(false);

// quick configure modification
const rustupDistServer = computed({
  get: () => managerConf.config.value.rustupDistServer?.[0] || '',
  set: (newValue: string) => {
    if (managerConf.config.value.rustupDistServer) {
      managerConf.config.value.rustupDistServer[0] = newValue;
    }
  }
});

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
  routerPush('/manager/progress');
}
</script>

<template>
  <div flex="~ col">
    <div>
      <span class="info-label">{{ $t('review_configuration') }}</span>
      <p class="sub-info-label">{{ $t('review_installation_hint') }}</p>
    </div>
    <base-card flex="1" mx="1vw" mb="7%" overflow="auto">
      <section>
        <b m="0" text='regular'>
          {{ $t('configuration') }}
          <u ml="1rem" text="active" cursor="pointer" @click="showConfigModifyPanel = true">{{ $t('change') }}</u>
        </b>
        <div m="1rem">
          <p flex gap="1rem">{{ $t('disable_ssl_cert_verification') }}: <b text='regular'>{{
            managerConf.config.value.insecure }}</b></p>
          <p flex gap="1rem">{{ $t('rustup_dist_server') }}: <b flex text='regular'>{{
            managerConf.config.value.rustupDistServer?.[0] }}<lock-indicator
                v-if="managerConf.config.value.rustupDistServer?.[1]" :hint="$t('config_disabled_reason')" /></b></p>
        </div>
      </section>
      
      <section>
        <b text='regular'>{{ $t('components_to_install') }}</b>
        <div m="1rem" v-for="item in labels" :key="item.label">
          <component-label :label="item.label" :oldVer="item.originVer" :newVer="item.targetVer" />
        </div>
      </section>

      <section v-if="obsoletedComponents.length > 0">
        <b text='regular'>{{ $t('components_to_remove') }}</b>
        <div m="1rem" v-for="item in obsoletedComponents">
          <component-label :label="item" />
        </div>
      </section>
    </base-card>

    <base-panel :show="showConfigModifyPanel" @close="showConfigModifyPanel = false" flex="~ col" overflow="auto" width="50%">
      <section>
        <h3 text-regular>{{ $t('system_configuration') }}</h3>
        <base-check-box v-model="managerConf.config.value.insecure" :title="$t('disable_ssl_cert_verification')"
          :hint="$t('disable_ssl_cert_verification_hint')" />
      </section>
      <br></br>
      <section>
        <h3 text-regular>{{ $t('source_configuration') }}</h3>
        <labeled-input v-model="rustupDistServer" :label="$t('rustup_dist_server')"
          :disabled="managerConf.config.value.rustupDistServer?.[1]" :disabledReason="$t('config_disabled_reason')"
          :hint="$t('rustup_dist_server_hint')" />
      </section>
    </base-panel>

    <page-nav-buttons :backLabel="$t('back')" :nextLabel="$t('install')" @back-clicked="routerBack"
      @next-clicked="handleNextClick" />
  </div>
</template>
