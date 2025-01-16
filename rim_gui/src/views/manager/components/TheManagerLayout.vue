<script setup lang="ts">
import { computed, onBeforeMount, onMounted, ref } from 'vue';
import { managerConf } from '@/utils';
import { useCustomRouter } from '@/router';

const appTitle = ref('');
const { isBack } = useCustomRouter();
const transitionName = computed(() => {
  if (isBack.value === true) return 'back';
  if (isBack.value === false) return 'push';
  return '';
});

onBeforeMount(() => managerConf.loadConf());

onMounted(() => {
  managerConf.appNameWithVersion().then((res) => {
    appTitle.value = res
  });
});
</script>

<template>
  <titlebar :title="appTitle" />
  <main absolute top="0" bottom="0" left="0" right="0" overflow-hidden style="margin-top: 40px;">
    <router-view v-slot="{ Component }">
      <transition :name="transitionName">
        <keep-alive>
          <component :is="Component" absolute w="full" style="height: calc(100% - 2rem)" />
        </keep-alive>
      </transition>
    </router-view>
  </main>
</template>
