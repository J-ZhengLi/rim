<script setup lang="ts">
import { managerConf, ManagerOperation } from '@/utils';
import KitCard from './components/KitCard.vue';
import { computed, onMounted, ref } from 'vue';
import Pagination from '@/components/Pagination.vue';
import { usePagination } from '@/utils/pagination';
import { event } from '@tauri-apps/api';
import { useCustomRouter } from '@/router';
import { CliPayload } from '@/utils/types/payloads';

const installedKit = computed(() => managerConf.getInstalled());
const kits = computed(() => managerConf.getKits());
const { current, size, total, list } = usePagination({
  data: kits.value,
  size: 6,
});
const loadingText = ref('');
const loaded = ref(false);

const { routerPush } = useCustomRouter();

onMounted(() => {
  event.listen('change-view', (event) => {
    let payload = event.payload as CliPayload;
    if (payload.command === 'Uninstall') {
      managerConf.setOperation(ManagerOperation.UninstallToolkit);
    }
    routerPush(payload.path);
  });
});
</script>

<template>
  <div overflow-y-auto>
    <loading-mask
      v-if="loadingText.length > 0"
      :text="loadingText"
      :finished="loaded"
    />
    <h2 mx="12px">更新和卸载</h2>
    <h3 mx="12px">已安装</h3>
    <kit-card
      v-if="installedKit"
      :key="installedKit.name"
      :kit="installedKit"
      :installed="true"
      mt="1rem"
    ></kit-card>
    <section overflow-auto flex="~ col">
      <h3 mx="12px" v-if="kits.length > 0">可用版本</h3>
      <kit-card
        v-for="kit in list"
        :key="kit.name"
        :kit="kit"
        :installed="false"
        mt="1rem"
      ></kit-card>
      <div flex="1"></div>
      <pagination
        :size="size"
        v-model="current"
        :total="total"
        hide-on-one-page
        show-jumper
        my="12px"
      />
    </section>
  </div>
</template>
