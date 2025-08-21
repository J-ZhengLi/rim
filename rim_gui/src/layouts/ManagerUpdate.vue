<template>
    <div text="center" v-if="!isUpdating">
        <h2 class="title">{{ $t('self_update_available') }}</h2>

        <span>{{ $t('ask_self_update') }}</span>
        <p mb="2rem">
            {{ $t('current') }}:
            <base-tag min-w="5rem">{{ curVer }}</base-tag>
            ➜
            {{ $t('latest') }}:
            <base-tag min-w="5rem" type="success">{{ newVer }}</base-tag>
        </p>

        <div flex="~ justify-between" style="gap: 2rem;">
            <base-button flex="1" theme="secondary" @click="onClose">{{ $t('cancel') }}</base-button>
            <base-button flex="1" theme="primary" @click="onUpdate">{{ $t('update') }}</base-button>
        </div>
    </div>
    <div v-else flex="~ col justify-center items-center">
        <span class="info-label">{{ mainProgressPayload?.message }}</span>
        <base-progress w="full" kind="spinner" />

        <span class="sub-info-label">{{ subProgressPayload?.message }}</span>
        <base-progress mt="0.5rem" w="full" h="4vh" :value="subProgress" :kind="subProgressPayload?.style.toString()"
            :length="subProgressPayload?.length" :transition="false" />
    </div>
</template>

<script setup lang="ts">
import { invokeCommand } from "@/utils";
import { onMounted, ref } from "vue";
import { event } from "@tauri-apps/api";
import { ProgressPayload } from "@/utils/types/payloads";

const isUpdating = ref(false);

const mainProgressPayload = ref<ProgressPayload | null>(null);
const subProgress = ref(0);
const subProgressPayload = ref<ProgressPayload | null>(null);

defineProps<{
    curVer: string;
    newVer: string;
}>();

const emit = defineEmits(['close']);

function onClose() {
    emit('close');
}

function onUpdate() {
    isUpdating.value = true;
    invokeCommand('self_update');
}

onMounted(async () => {
    event.listen('progress:main-start', (event) => {
        const payload = event.payload as ProgressPayload;
        mainProgressPayload.value = payload;
    });

    event.listen('progress:main-end', (event) => {
        if (typeof event.payload === 'string' && mainProgressPayload.value) {
            mainProgressPayload.value = {
                ...mainProgressPayload.value,
                message: event.payload
            };
        }
    });

    event.listen('progress:sub-start', (event) => {
        const payload = event.payload as ProgressPayload;
        subProgress.value = 0;
        subProgressPayload.value = payload;
    });

    event.listen('progress:sub-update', (event) => {
        if (typeof event.payload === 'number') {
            subProgress.value += event.payload;
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
});
</script>
