<script setup lang="ts">
import type { Ref } from 'vue';
import { event } from '@tauri-apps/api';
import { message } from '@tauri-apps/api/dialog';
import { nextTick, onMounted, ref, watch } from 'vue';
import { useCustomRouter } from '@/router/index';
import { invokeCommand, invokeLabelList } from '@/utils/index';

const { routerPush } = useCustomRouter();
const progress = ref(0);
const output: Ref<string[]> = ref([]);
const scrollBox: Ref<HTMLElement | null> = ref(null);
const labels: Ref<Record<string, string>> = ref({});
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

  invokeLabelList(['show_details', 'installing', 'install_finished']).then((res) => {
    labels.value = res;
  });

  intervalId = window.setInterval(() => {
    dots.value = '.'.repeat(dotCount.value);
    dotCount.value = (dotCount.value + 1) % 4;
  }, 500);

  window.setInterval(() => {
    if (progress.value < 100) {
      progress.value += 1;
      output.value.push("info: this is a test message " + progress.value);
    }
  }, 100);
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
    <span class="info-label">{{ progress >= 100 ? labels.install_finished : labels.installing }}{{ dots }}</span>
    <div mx="1vw" mt="2vh">
      <base-progress w="full" h="4vh" :percentage="progress" />
    </div>
    <base-details my="2vh" mx="0.5vw" :title="labels.show_details">
      <base-card h="43vh" mx="0.5vw" my="0.5vh">
        <div ref="scrollBox" flex="1" overflow="auto" h="full">
          <p my="0.5rem" v-for="item in output" :key="item">{{ item }}</p>
        </div>
      </base-card>
    </base-details>
    <page-nav-buttons :hideBack="true" :hideNext="progress < 100" @next-clicked="() => routerPush('/installer/finish')" />
  </div>
</template>
