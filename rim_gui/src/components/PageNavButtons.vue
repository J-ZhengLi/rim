<template>
    <div flex="~ items-center" position="fixed" bottom="7%">
      <base-button v-if="backLabel" theme="secondary-btn" position="fixed" left="0" style="transform: translateX(30%);"
        @click="backClicked">{{ backLabel }}</base-button>
      <base-button v-if="nextLabel" theme="primary" position="fixed" right="0" style="transform: translateX(-30%);"
        @click="nextClicked">{{ nextLabel }}</base-button>
    </div>
</template>

<script setup lang="ts">
defineProps<{
    backLabel?: string,
    nextLabel?: string,
}>();

const emit = defineEmits<{
  (e: 'back-clicked'): void;
  (e: 'next-clicked'): void;
}>();

const backClicked = () => emit('back-clicked');
const nextClicked = () => emit('next-clicked');
</script>
