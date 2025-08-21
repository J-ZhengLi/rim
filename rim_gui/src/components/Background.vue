<template>
    <div class="with-background" id="background">
        <div v-if="animated">
            <div v-for="(bubble, index) in bubbles" :key="index" class="bubble" :style="{
                '--x': `${bubble.x}px`,
                '--y': `${bubble.y}px`,
                '--size': `${bubble.size}px`,
                '--blur': `${bubble.blur}px`,
                '--hue': bubble.hue
            }"></div>
        </div>
    </div>
</template>

<script lang="ts" setup>
import { onMounted, onUnmounted, Ref, ref } from 'vue';

const props = defineProps({
  animated: {
    type: Boolean,
    default: true,
  }
});

// bubble controls
const bubbles: Ref<Bubble[]> = ref([]);
const containerSize = ref({ width: 0, height: 0 });
let animationFrame: number | null = null;
interface Bubble {
  x: number;
  y: number;
  size: number;
  blur: number;
  hue: number;
  vx: number;
  vy: number;
}

// Initialize bubbles with random properties
function initBubbles() {
  const count = 3; // Number of bubbles
  // a random scale within [-1, -0.5] and [0.5, 1] to prevent frozen bubbles
  const randomSpeedScale = (): number => {
    const rand = Math.random();
    return rand < 0.5 ? 0.5 + rand : -1 + rand - 0.5;
  };
  const size = containerSize.value.height / 1.5 + Math.random() * 100;
  bubbles.value = Array.from({ length: count }, () => ({
    x: Math.random() * (containerSize.value.width - size),
    y: Math.random() * (containerSize.value.height - size),
    size: size,
    blur: 5 + Math.random() * 15, // slight random blur
    hue: 185 + Math.random() * 35,  // blueish hue
    vx: randomSpeedScale() * 0.25, // Horizontal velocity
    vy: randomSpeedScale() * 0.25, // Vertical velocity
  }));
}

// Update bubble positions and handle collisions
function updateBubbles() {
  bubbles.value.forEach(bubble => {
    // Update position
    bubble.x += bubble.vx;
    bubble.y += bubble.vy;

    // Boundary collision
    const bubbleRadius = bubble.size / 2;
    if (bubble.x + bubbleRadius < 0 || bubble.x + bubbleRadius > containerSize.value.width) bubble.vx *= -1;
    if (bubble.y < 0 || bubble.y + bubbleRadius > containerSize.value.height) bubble.vy *= -1;
  });
}

// Animation loop
function animate() {
  updateBubbles();
  animationFrame = requestAnimationFrame(animate);
}

// Handle window resize
function handleResize() {
  if (!props.animated) {
    return;
  }
  containerSize.value = {
    width: window.innerWidth,
    height: window.innerHeight
  };
  initBubbles();
}

onMounted(() => {
  console.log("animated background mounted, make sure this message only appear once.");
  window.addEventListener('resize', handleResize);
  handleResize();
  animate();
});

onUnmounted(() => {
  window.removeEventListener('resize', handleResize);
  if (animationFrame) {
    cancelAnimationFrame(animationFrame);
  }
});
</script>

<style lang="css" scoped>
.with-background {
  background: linear-gradient(to bottom,
      white,
      rgba(255, 255, 255, 0.1),
      rgb(210, 231, 255));
  border: 1px solid rgb(166, 166, 166);
  box-sizing: border-box;
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
  z-index: -1;
}

.bubble {
  position: absolute;
  top: 0;
  left: 0;
  /* TODO: how to make this size scales to the container */
  width: var(--size);
  height: var(--size);
  border-radius: 50%;
  background: hsl(var(--hue), 100%, 70%);
  filter: blur(var(--blur));
  opacity: 0.6;
  transform:
    translateX(var(--x)) translateY(var(--y)) translateZ(0);
  will-change: transform;
}
</style>
