<template>
    <div flex="~ items-center" position="fixed" bottom="7%">
      <base-button v-if="!hideBack" theme="secondary-btn" position="fixed" left="0" style="transform: translateX(30%);"
        @click="backClicked">{{ backBtnLbl }}</base-button>
      <base-button v-if="!hideNext" theme="primary" position="fixed" right="0" style="transform: translateX(-30%);"
        @click="nextClicked">{{ nextBtnLbl }}</base-button>
    </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

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
    backBtnLbl.value = props.backLabel ? props.backLabel : t('back');
}, { immediate: true });
watch(() => props.nextLabel, () => {
    nextBtnLbl.value = props.nextLabel ? props.nextLabel : t('next');
}, { immediate: true });
</script>
