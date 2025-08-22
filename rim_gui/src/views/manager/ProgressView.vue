<script setup lang="ts">
import type { Ref } from 'vue';
import { event } from '@tauri-apps/api';
import { nextTick, onMounted, ref, watch } from 'vue';
import { useCustomRouter } from '@/router/index';
import { ProgressPayload } from '@/utils/types/payloads';
import { managerConf, invokeCommand, ManagerOperation } from '@/utils';

const { routerBack, routerPush, routerPushAndClearCache } = useCustomRouter();

// ===== progress bar related section =====
const progress = ref(0);
const mainProgressPayload = ref<ProgressPayload | null>(null);
const showSubProgress = ref(false);
const hideSubProgressTimeout = ref<NodeJS.Timeout | null>(null);
const subProgress = ref(0);
const subProgressPayload = ref<ProgressPayload | null>(null);
// ===== progress bar related section =====

const output: Ref<string[]> = ref([]);
const scrollBox: Ref<HTMLElement | null> = ref(null);
const showBackButton = ref(false);

// TODO: We can merge managerConf and installConf together, then
// we can use this view for both install/update/uninstall since
// the layout of this view is technically the same.
async function performOperation() {
  try {
    switch (managerConf.getOperation()) {
      case ManagerOperation.UninstallAll:
        await invokeCommand('uninstall_toolkit', { remove_self: true })
        break;
      case ManagerOperation.UninstallToolkit:
        await invokeCommand('uninstall_toolkit', { remove_self: false })
        break;
      case ManagerOperation.Update:
        await invokeCommand('install_toolkit', {
          componentsList: managerConf.getTargetComponents(),
          config: managerConf.config.value,
        });
        break;
      default:
        break;
    }
  } catch (_) {
    showBackButton.value = true;
  }
}

function back() {
    showBackButton.value = false;
    progress.value = 0;
    mainProgressPayload.value = null;

    routerBack();
}

async function complete() {
  if (managerConf.getOperation() === ManagerOperation.UninstallAll) {
    routerPush('/manager/complete');
  } else {
    await managerConf.reloadKits();
    routerPushAndClearCache('/manager');
  }
}

onMounted(async () => {
  // main progress bar events
  event.listen('progress:main-start', (event) => {
    const payload = event.payload as ProgressPayload;
    progress.value = 0;
    mainProgressPayload.value = payload;
  });

  event.listen('progress:main-update', (event) => {
    if (typeof event.payload === 'number') {
      progress.value += event.payload;
    }
  });

  event.listen('progress:main-end', (event) => {
    if (typeof event.payload === 'string' && mainProgressPayload.value) {
      mainProgressPayload.value = {
        ...mainProgressPayload.value,
        message: event.payload
      };
    }
  });

  // sub progress bar events
  event.listen('progress:sub-start', (event) => {
    const payload = event.payload as ProgressPayload;
    subProgress.value = 0;
    subProgressPayload.value = payload;
  });

  event.listen('progress:sub-update', (event) => {
    if (typeof event.payload === 'number') {
      subProgress.value = event.payload;
    }
  });

  event.listen('progress:sub-end', (event) => {
    if (typeof event.payload === 'string' && subProgressPayload.value) {
      subProgressPayload.value = {
        ...subProgressPayload.value,
        message: event.payload
      };
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

  // finish event listener (in case some times the progress does not reach 100 by mis-calculation)
  event.listen('on-complete', () => progress.value = 100);

  // NB (J-ZhengLi): This invoke call MUST be called after registering event listeners
  // otherwise the events sent from backend will be lost.
  await performOperation();
});

watch(subProgress, (val) => {
  // Manually resetting the sub-progress once its finished.
  // Because not every operation has a certain progress,
  // such as installing toolchain via `rustup`, which we don't know how long it will take.
  // Ideally we can use a spinner like in CLI mode. But it might now look good
  // if the bar keeps changing styles back and forth.
  // Therefore it's probably better to hide it for now.
  if (subProgressPayload.value?.length && val >= subProgressPayload.value.length) {
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

// re-run if operation changed and view is reused
watch(() => managerConf.getOperation(), (newOp) => {
  console.log('showing progress for', newOp);
  performOperation();
});
</script>

<template>
  <div flex="~ col">
    <span class="info-label">{{ mainProgressPayload?.message }}</span>
    <base-progress mt="2vh" w="full" h="4vh" :value="progress" kind="percentage"
      :length="mainProgressPayload?.length" />

    <div v-if="showSubProgress">
      <p class="sub-info-label">{{ subProgressPayload?.message }}</p>
      <base-progress w="full" h="4vh" :value="subProgress" :kind="subProgressPayload?.style.toString()"
        :length="subProgressPayload?.length" :transition="false" />
    </div>
    <base-details my="2vh" mx="0.5vw" :title="$t('show_details')">
      <base-card h="40vh" mx="0.5vw" my="0.5vh">
        <div ref="scrollBox" flex="1" overflow="auto" h="full">
          <p my="0.5rem" v-for="item in output" :key="item">{{ item }}</p>
        </div>
      </base-card>
    </base-details>
    <page-nav-buttons
      :nextLabel="progress < 100 ? undefined : $t('next')" :backLabel="showBackButton ? $t('back') : ''"
      @next-clicked="complete"
      @back-clicked="back" />
  </div>
</template>
