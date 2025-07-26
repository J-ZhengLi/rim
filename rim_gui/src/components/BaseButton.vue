<script setup lang="ts">
import { computed } from 'vue';

// Define props for the component
const props = defineProps({
  theme: {
    type: String,
    default: 'default', // Default theme
  },
  disabled: {
    type: Boolean,
    default: false, // Button is enabled by default
  },
});

// Computed class for dynamic theme application
const themeClasses = computed(() => {
  switch (props.theme) {
    case 'primary':
      return 'bg-primary text-white active:bg-deep-primary';
    case 'secondary-btn':
      return 'bg-secondary-btn text-header';
    // Add more themes as needed
    default:
      return 'bg-gray-200 text-header border-gray-400 active:bg-gray-300';
  }
});
</script>

<template>
  <button p="x-3% y-1%" :class="[
    themeClasses,
    'rounded-[30vw] b-none hover:op-80', // Common classes
    { 'cursor-pointer': !disabled }, // Disabled styles
    { 'opacity-50 cursor-not-allowed': disabled }, // Disabled styles
  ]" :disabled="disabled">
    <slot></slot>
  </button>
</template>

<style scoped>
button {
  font-size: clamp(100%, 3vh, 20px);
  box-shadow: 0 0 0 1px rgba(255, 255, 255, .6), 0 8px 16px rgba(0, 0, 0, .12);
  font-weight: bold;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  transition:
    background-color 0.3s,
    border-color 0.3s;
}
</style>
