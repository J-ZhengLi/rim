<script setup lang="ts">
import { computed, onBeforeMount } from 'vue';
import { useCustomRouter } from '@/router';
import { installConf } from '@/utils';
import { useRoute } from 'vue-router';
import PageIndicator from './PageIndicator.vue';

const route = useRoute();
const { isBack } = useCustomRouter();
const isHome = computed(() => route.name === 'Home');

const transitionName = computed(() => {
  if (isBack.value === true) return 'back';
  if (isBack.value === false) return 'push';
  return '';
});

onBeforeMount(() => installConf.loadAll());
</script>

<template>
  <div flex="~ items-stretch" top-0 left-0 bottom-0 right-0>
    <PageIndicator v-if="!isHome" />
    <main p="4vh" flex="1" overflow="hidden" absolute top="0" right="0" bottom="0"
      :style="{ left: '0', top: isHome ? '0' : '4vh' }">
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
  </div>
</template>

<style>
.push-enter-active,
.push-leave-active,
.back-enter-active,
.back-leave-active {
  transition: all 0.5s ease;
}

/* 页面前进 */
.push-enter-from {
  right: -100%;
  opacity: 0.5;
}

.push-enter-to {
  right: 0;
  opacity: 1;
}

.push-leave-from {
  left: 0;
  opacity: 1;
}

.push-leave-to {
  left: -100%;
  opacity: 0;
}

/* 页面返回 */
.back-enter-from {
  left: -100%;
  opacity: 0.5;
}

.back-enter-to {
  left: 0;
  opacity: 1;
}

.back-leave-from {
  right: 0;
  opacity: 1;
}

.back-leave-to {
  right: -100%;
  opacity: 0;
}
</style>
