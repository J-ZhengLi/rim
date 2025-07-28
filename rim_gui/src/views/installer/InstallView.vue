<script setup lang="ts">
import type { Ref } from 'vue';
import { event } from '@tauri-apps/api';
import { nextTick, onMounted, ref, watch } from 'vue';
import { useCustomRouter } from '@/router/index';
import { ProgressPayload, ProgressStyle } from '@/utils/types/payloads';

const { routerPush } = useCustomRouter();

// ===== progress bar related section =====
const progress = ref(0);
const progressMsg = ref('Main Progress');
const showSubProgress = ref(false);
const hideSubProgressTimeout = ref<NodeJS.Timeout | null>(null);
const subProgress = ref(0);
const subProgressMsg = ref('Secondary Progress');
const subProgressLen = ref<number | undefined>(1024000);
const subProgressStyle = ref(ProgressStyle.Bytes);
// ===== progress bar related section =====

const output: Ref<string[]> = ref([]);
const scrollBox: Ref<HTMLElement | null> = ref(null);

onMounted(() => {
  // main progress bar events
  event.listen('progress:main-start', (event) => {
    const payload = event.payload as ProgressPayload;
    progressMsg.value = payload.message;
  });

  event.listen('progress:main-update', (event) => {
    if (typeof event.payload === 'number') {
      progress.value += event.payload;
    }
  });

  event.listen('progress:main-end', (event) => {
    if (typeof event.payload === 'string') {
      progressMsg.value = event.payload;
    }
  });

  // sub progress bar events
  event.listen('progress:sub-start', (event) => {
    const payload = event.payload as ProgressPayload;
    subProgress.value = 0;
    subProgressMsg.value = payload.message;
    subProgressLen.value = payload.length;
    subProgressStyle.value = payload.style;
  });

  event.listen('progress:sub-update', (event) => {
    if (typeof event.payload === 'number') {
      subProgress.value = event.payload;
    }
  });

  event.listen('progress:sub-end', (event) => {
    if (typeof event.payload === 'string') {
      subProgressMsg.value = event.payload;
    }
  });

  // detailed message event
  event.listen('update-message', (event) => {
    if (typeof event.payload === 'string') {
      event.payload.split('\n').forEach((line) => {
        output.value.push(line);
      });
    }
  });

  // finish event listener
  event.listen('on-complete', () => {
    setTimeout(() => {
      routerPush('/installer/finish');
    }, 3000);
  });
});

watch(subProgress, (val) => {
  // Manually resetting the sub-progress once its finished.
  // Because not every operation has a certain progress,
  // such as installing toolchain via `rustup`, which we don't know how long it will take.
  // Ideally we ca use a spinner like in CLI mode. But it might now look good
  // if the bar keeps changing styles back and forth. Therefore it's probably hide it for now.
  if (subProgressLen.value && val >= subProgressLen.value) {
    hideSubProgressTimeout.value = setTimeout(() => showSubProgress.value = false, 3000);
  } else {
    if (hideSubProgressTimeout.value) {
      clearTimeout(hideSubProgressTimeout.value);
    }
    showSubProgress.value = true;
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
});
</script>

<template>
  <div flex="~ col">
    <span class="info-label">{{ progressMsg }}</span>
    <base-progress mt="2vh" w="full" h="4vh" :value="progress" type="percentage" />

    <div v-if="showSubProgress">
      <p class="sub-info-label">{{ subProgressMsg }}</p>
      <base-progress w="full" h="4vh" :value="subProgress" :style="subProgressStyle.toString()" :length="subProgressLen"
        :transition="false" />
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
