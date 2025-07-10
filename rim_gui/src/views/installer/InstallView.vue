<script setup lang="ts">
import type { Ref } from 'vue';
import { event } from '@tauri-apps/api';
import { message } from '@tauri-apps/api/dialog';
import { nextTick, onMounted, ref, watch } from 'vue';
import { useCustomRouter } from '@/router/index';
import { invokeCommand } from '@/utils/index';

const { routerPush } = useCustomRouter();
const progress = ref(0);
const output: Ref<string[]> = ref([]);
const scrollBox: Ref<HTMLElement | null> = ref(null);
const dotCount = ref(0);
const dots = ref('');

let intervalId: number | null = null;

onMounted(() => {
  event.listen('update-progress', (event) => {
    if (typeof event.payload === 'number') {
      progress.value = event.payload;
    }
  });

  event.listen('update-message', (event) => {
    if (typeof event.payload === 'string') {
      event.payload.split('\n').forEach((line) => {
        output.value.push(line);
      });
    }
  });

  event.listen('on-complete', () => {
    setTimeout(() => {
      routerPush('/installer/finish');
    }, 3000);
  });

  event.listen('on-failed', (event) => {
    if (typeof event.payload === 'string') {
      output.value.push(event.payload);
      message(event.payload, { title: '错误', type: 'error' }).then(() =>
        invokeCommand('close_window')
      );
    }
  });

  intervalId = window.setInterval(() => {
    dots.value = '.'.repeat(dotCount.value);
    dotCount.value = (dotCount.value + 1) % 4;
  }, 500);
});

watch(progress, (newValue) => {
  if (newValue >= 100) {
    if (intervalId) clearInterval(intervalId);
    dots.value = '!';
  }
});

watch(output.value, () => {
  nextTick(() => {
    // scroll to bottom
    if (scrollBox.value) {
      scrollBox.value.scrollTo({
        top: scrollBox.value.scrollHeight,
        behavior: 'smooth'
      });
    }
  });
})
</script>

<template>
  <div flex="~ col">
    <span class="info-label">{{ $t(progress >= 100 ? 'install_finished' : 'installing') }}{{ dots }}</span>
    <p class="sub-info-label">{{ $t('installing_hint') }}</p>
    <div mx="1vw">
      <base-progress w="full" h="4vh" :percentage="progress" />
    </div>
    <base-details my="2vh" mx="0.5vw" :title="$t('show_details')">
      <base-card h="40vh" mx="0.5vw" my="0.5vh">
        <div ref="scrollBox" flex="1" overflow="auto" h="full">
          <p my="0.5rem" v-for="item in output" :key="item">{{ item }}</p>
        </div>
      </base-card>
    </base-details>
    <page-nav-buttons :nextLabel="progress < 100 ? undefined : $t('next')" :hideNext="progress < 100"
      @next-clicked="() => routerPush('/installer/finish')" />
  </div>
</template>
