<script setup lang="ts">
import { RouterView } from 'vue-router';
</script>

<template>
  <div>
    <RouterView />
  </div>
</template>

<style>
:root {
  margin: 0;
  padding: 0;
  font-family:
    -apple-system,
    BlinkMacSystemFont,
    Segoe UI,
    Helvetica,
    Arial,
    sans-serif,
    Apple Color Emoji,
    Segoe UI Emoji,
    Segoe UI Symbol;
  font-size: 14px;
  line-height: 24px;
  letter-spacing: normal;
  font-weight: 400;
  --uno: bg-back c-header;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}
</style>
