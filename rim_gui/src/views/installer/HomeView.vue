<script lang="ts" setup>
import { onBeforeMount, onMounted, onUnmounted, Ref, ref } from 'vue';
import { useCustomRouter } from '@/router/index';
import { installConf, invokeCommand, invokeLabelList } from '@/utils/index';

const { routerPush } = useCustomRouter();

const toolkitName = ref('');
const labels = ref<Record<string, string>>({});

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
  const count = 5; // Number of bubbles
  // a random scale within [-1, -0.5] and [0.5, 1] to prevent frozen bubbles
  const randomSpeedScale = (): number => {
    const rand = Math.random();
    return rand < 0.5 ? 0.5 + rand : -1 + rand - 0.5;
  };
  const size = 400 + Math.random() * 100; // 400-500px
  bubbles.value = Array.from({ length: count }, () => ({
    x: Math.random() * (containerSize.value.width - size),
    y: Math.random() * (containerSize.value.height - size),
    size: size,
    blur: 5 + Math.random() * 15, // slight blur between 5-20px
    hue: 180 + Math.random() * 50,  // blueish hue
    vx: randomSpeedScale() * 0.5, // Horizontal velocity
    vy: randomSpeedScale() * 0.5, // Vertical velocity
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
    if (bubble.y + bubbleRadius < 0 || bubble.y + bubbleRadius > containerSize.value.height) bubble.vy *= -1;
  });
}

// Animation loop
function animate() {
  updateBubbles();
  animationFrame = requestAnimationFrame(animate);
}

// Handle window resize
function handleResize() {
  containerSize.value = {
    width: window.innerWidth,
    height: window.innerHeight
  };
  initBubbles();
}

function handleInstallClick(custom: boolean) {
  installConf.setCustomInstall(custom);
  routerPush(custom ? '/installer/folder' : '/installer/confirm');
}

function selectOtherEdition() {
  console.log("TODO: show a page to load toolkit form url or path");
}

onBeforeMount(() => installConf.loadManifest());

onMounted(() => {
  const labelKeys = [
    'install',
    'install_other_edition',
  ];
  invokeLabelList(labelKeys).then((results) => {
    labels.value = results;
  });

  invokeCommand('toolkit_name').then((lb) => {
    if (typeof lb === 'string') {
      toolkitName.value = lb;
    }
  });

  invokeCommand('get_build_cfg_locale_str', { key: 'vendor' }).then((res) => {
    if (typeof res === 'string') {
      labels.value.vendor = res
    }
  });

  invokeCommand('get_build_cfg_locale_str', { key: 'content_source' }).then((res) => {
    if (typeof res === 'string') {
      labels.value.content_source = res
    }
  });

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

<template>
  <div class="with-background" flex="~ col items-center" w="full">
    <div w="full" text="center" class="bubble-container">
      <div v-for="(bubble, index) in bubbles" :key="index" class="bubble" :style="{
        '--x': `${bubble.x}px`,
        '--y': `${bubble.y}px`,
        '--size': `${bubble.size}px`,
        '--blur': `${bubble.blur}px`,
        '--hue': bubble.hue
      }"></div>
      <base-card h="50%" w="60%">
        <div flex="~ col items-center" h="full">
          <div class="toolkit-info">
            <div c="darker-secondary" font="bold" text="4vh">{{ toolkitName }}</div>
            <div c="secondary" text="3.5vh">{{ installConf.version }}</div>
          </div>
          <base-button theme="primary" font="bold" w="20vw" h="7vh" @click="handleInstallClick(true)">{{ labels.install }}</base-button>
          <span @click="selectOtherEdition" c="secondary" mt="10px" cursor-pointer underline>{{ labels.install_other_edition }}</span>
        </div>
      </base-card>
    </div>
    <div class="content-disclaimer">{{ labels.content_source }}</div>
  </div>
</template>

<style lang="css" scoped>
.with-background {
  background: linear-gradient(to bottom,
      white,
      rgba(255, 255, 255, 0.1),
      rgb(158, 203, 255));
  background-size: 100% auto;
  border: 1px solid rgb(166, 166, 166);
  box-sizing: border-box;
}

.toolkit-info {
  margin-top: 12%;
  margin-bottom: 10%;
  display: flex;
  flex-direction: column;
  gap: 5vh;
}

.content-disclaimer {
  --uno: c-secondary;
  position: fixed;
  font-size: 14px;
  bottom: 3vh;
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
