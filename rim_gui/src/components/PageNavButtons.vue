<template>
    <div flex="~ items-center" position="fixed" bottom="7%">
      <base-button v-if="!hideBack" theme="secondary-btn" position="fixed" left="0" style="transform: translateX(30%);"
        @click="backClicked">{{ backBtnLbl }}</base-button>
      <base-button v-if="!hideNext" theme="primary" position="fixed" right="0" style="transform: translateX(-30%);"
        @click="nextClicked">{{ nextBtnLbl }}</base-button>
    </div>
</template>

<script setup lang="ts">
import { invokeLabel } from '@/utils';
import { ref, watch } from 'vue';

const props = defineProps({
    backLabel: String,
    nextLabel: String,
    hideBack: {
        type: Boolean,
        default: false,  
    },
    hideNext: {
        type: Boolean,
        default: false,
    }
});

const emit = defineEmits<{
  (e: 'back-clicked'): void;
  (e: 'next-clicked'): void;
}>();

const backClicked = () => emit('back-clicked');
const nextClicked = () => emit('next-clicked');

const backBtnLbl = ref('');
const nextBtnLbl = ref('');

// watch prop changes for async label loading
watch(() => props.backLabel, () => {
    if (props.backLabel) {
        backBtnLbl.value = props.backLabel;
    } else {
        invokeLabel('back').then((res) => {
            backBtnLbl.value = res;
        });
    }
}, { immediate: true });
watch(() => props.nextLabel, () => {
    if (props.nextLabel) {
        nextBtnLbl.value = props.nextLabel;
    } else {
        invokeLabel('next').then((res) => {
            nextBtnLbl.value = res;
        });
    }
}, { immediate: true });
</script>
