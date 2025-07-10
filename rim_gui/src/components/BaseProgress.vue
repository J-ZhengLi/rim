<template>
  <div flex="~ items-center justify-between">
    <div class="progress-bar">
      <div class="progress-fill" :style="{ width: props.percentage + '%' }"></div>
    </div>
    <div class="progress-label" text-end>{{ progressFormat(percentage) }}</div>
  </div>
</template>

<script setup lang="ts">
const props = defineProps({
  percentage: {
    type: Number,
    required: true,
    validator: (value: number) => value >= 0 && value <= 100,
  },
});

function progressFormat(value: number) {
  return value.toFixed(2).padStart(5, '0') + '%';
}
</script>

<style scoped>
.progress-bar {
  width: 100%;
  height: 100%;
  border-radius: 24px;
  overflow: hidden;
  background: rgba(255, 255, 255, .4);
  box-shadow: 0 0 0 2px rgba(255, 255, 255, .6), 0 16px 32px rgba(0, 0, 0, .12);
  backdrop-filter: blur(25px);
  -webkit-backdrop-filter: blur(25px);
  outline: 0;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(270deg,
      #5b98d8,
      #a0dcff,
      #5b98d8);
  background-size: 200% 100%;
  animation: gradientMove 3s linear infinite;
  transition: width 0.5s ease-in-out;
}

.progress-label {
  margin-left: 1rem;
  font-size: clamp(80%, 2.3vh, 20px);
}

@keyframes gradientMove {
  0% {
    background-position: 0% 50%;
  }

  100% {
    background-position: -200% 50%;
  }
}
</style>
