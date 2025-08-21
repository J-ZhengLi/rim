<template>
  <div flex="~ items-center justify-between">
    
    <div v-if="kind === 'spinner'" w="full" my="1rem" flex="~ justify-center">
      <spinner v-if="kind === 'spinner'" size="30px" color="blue" class="progress-spinner" />
    </div>
    <div v-else class="progress-bar">
      <div class="progress-fill"
        :class="{ 'progress-transition': transition }"
        :style="{ width: valuePercentage(kind, value, length) + '%' }"></div>
      <div class="progress-label" text-end>{{ progressFormat(kind, value, length) }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { PropType } from 'vue';

type ProgressKind = 'percentage' | 'len' | 'bytes' | 'spinner' | 'hidden';

defineProps({
  value: {
    type: Number,
    default: 0,
  },
  length: {
    type: Number,
    default: 0,
  },
  kind: {
    type: String as PropType<ProgressKind>,
    default: 'percentage',
    validator: (value: string) => ['percentage', 'len', 'bytes', 'spinner', 'hidden'].includes(value)
  },
  transition: {
    type: Boolean,
    default: true,
  }
});

// calculate the progress bar fill percentage
function valuePercentage(kind: ProgressKind, value: number, length?: number): number {
  switch (kind) {
    case 'percentage':
      return value;
    case 'len':
    case 'bytes':
      if (length) {
        return value / length * 100;
      } else {
        return 0;
      }
    default:
      return 0;
  }
}

function progressFormat(kind: ProgressKind, value: number, length?: number): string {
  switch (kind) {
    case 'bytes':
      return `${formatBytes(value)}${length ? ' | ' + formatBytes(length) : ''}`;
    case 'len':
      return `${value}${length ? ' | ' + length : ''}`;
    case 'percentage':
      return value.toFixed(2) + '%';
    default:
      // spinner and hidden progess bar doesn't need value labels
      return '';
  }
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0B';

  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  const value = bytes / Math.pow(1024, i);

  const rounded = value % 1 === 0 ? value.toString() : value.toFixed(2);

  return `${rounded}${sizes[i]}`;
}
</script>

<style scoped>
.progress-spinner {
  height: 100%;
}

.progress-bar {
  width: 100%;
  height: 100%;
  border-radius: 24px;
  overflow: hidden;
  background: rgba(255, 255, 255, .4);
  box-shadow: 0 0 0 2px rgba(255, 255, 255, .6), 0 16px 32px rgba(0, 0, 0, .12);
  backdrop-filter: blur(25px);
  outline: 0;
  margin-inline: 1vw;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(270deg,
      #5b98d8,
      #a0dcff,
      #5b98d8);
  background-size: 200% 100%;
  animation: gradientMove 3s linear infinite;
}

.progress-transition {
  transition: width 0.5s ease-in-out;
}

.progress-label {
  position: absolute;
  display: flex;
  inset: 0;
  align-items: center;
  justify-content: center;
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
