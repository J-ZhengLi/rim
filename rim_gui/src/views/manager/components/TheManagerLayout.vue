<script setup lang="ts">
import { computed, onBeforeMount } from 'vue';
import { managerConf } from '@/utils';
import { useCustomRouter } from '@/router';

const { isBack } = useCustomRouter();
const transitionName = computed(() => {
  if (isBack.value === true) return 'back';
  if (isBack.value === false) return 'push';
  return '';
});

onBeforeMount(() => managerConf.load());
</script>

<template>
  <main p="4vh" flex="1" overflow="hidden" absolute top="0" right="0" left="0" bottom="0" class="main">
    <div h-full relative>
      <router-view v-slot="{ Component }">
        <transition :name="transitionName">
          <keep-alive>
            <component :is="Component" absolute w="full" h="full" />
          </keep-alive>
        </transition>
      </router-view>
    </div>
  </main>
</template>

<style lang="css" scoped>
.main {
  box-sizing: border-box;
}
</style>
