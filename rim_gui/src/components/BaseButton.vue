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
    case 'secondary':
      return 'bg-secondary-btn text-regular';
    // Add more themes as needed
    default:
      return 'bg-gray-200 text-regular border-gray-400 active:bg-gray-300';
  }
});
</script>

<template>
  <button :class="[themeClasses, disabled ? 'button-disabled' : 'button-active']" :disabled="disabled">
    <slot></slot>
  </button>
</template>

<style scoped>
button {
  padding: 3% 2.5%;
  font-size: clamp(100%, 3vh, 20px);
  border-radius: 100px;
  border: none;
  box-shadow: 0 0 0 1px rgba(255, 255, 255, .6), 0 8px 16px rgba(0, 0, 0, .12);
  font-weight: bold;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  transition:
    background-color 0.3s,
    border-color 0.3s;
}

.button-active {
  cursor: pointer;
}

.button-active:hover {
  opacity: 90%;
  box-shadow: 0 0 0 1px rgba(91, 155, 213, .12), 0 6px 12px rgba(91, 155, 213, .8);
}

.button-disabled {
  cursor: not-allowed;
  opacity: 50%;
}
</style>
