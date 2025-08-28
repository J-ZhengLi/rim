<script setup lang="ts">
import { useCustomRouter } from '@/router/index';
import { managerConf, ManagerOperation } from '@/utils';
import { computed, ref, watch } from 'vue';
import Label from './components/Label.vue';
const { routerBack, routerPush } = useCustomRouter();

const isUninstallManger = ref(false);
const installDir = computed(() => managerConf.config.value.path);

watch(isUninstallManger, (val: boolean) => {
  if (val) {
    managerConf.setOperation(ManagerOperation.UninstallAll);
  } else {
    managerConf.setOperation(ManagerOperation.UninstallToolkit);
  }
});

const installed = computed(() => managerConf.getInstalled());

function handleUninstall() {
  routerPush('/manager/progress');
}
</script>
<template>
  <div flex="~ col" w="full" h="full">
    <span class="info-label">{{ $t('uninstall') }}</span>
    <p class="sub-info-label">{{ $t('uninstall_confirmation', { name: installed?.name || $t('toolkit') }) }}</p>
    <base-card flex="1" mx="1vw" mb="1vh" overflow="auto">
      <section>
        <b m="0" text='regular'>{{ $t('installation_path') }}</b>
        <div m="1rem">
          <p flex gap="1rem">{{ installDir }}</p>
        </div>
      </section>

      <section>
        <b text='regular'>{{ $t('components_to_remove') }}</b>
        <div m="1rem" v-for="item in installed?.components" :key="item.id">
          <Label :label="item.displayName" :old-ver="item.version"></Label>
        </div>
      </section>
    </base-card>
    <div m="l-1vw t-0.5rem" mb="6%">
      <base-check-box v-model="isUninstallManger" :title="$t('uninstall_self_question')" />
    </div>
    <page-nav-buttons :backLabel="$t('cancel')" :nextLabel="$t('uninstall')" @back-clicked="routerBack" @next-clicked="handleUninstall" />
  </div>
</template>
