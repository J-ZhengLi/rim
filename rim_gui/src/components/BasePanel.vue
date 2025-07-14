<template>
    <transition name="panel">
        <div v-if="props.show" class="panel-backdrop" @click.self="emit('close')">
            <div class="panel-content" :style="{
                width: width,
                height: height,
            }">
                <slot></slot>
            </div>
        </div>
    </transition>
</template>

<script setup lang="ts">
const props = defineProps({
    show: {
        type: Boolean,
        required: true
    },
    width: {
        type: String,
        default: 'auto'
    },
    height: {
        type: String,
        default: 'auto'
    },
});

const emit = defineEmits(['close']);
</script>

<style scoped>
.panel-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    backdrop-filter: blur(25px);
    -webkit-backdrop-filter: blur(25px);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 999;
}

.panel-content {
    background: rgba(255, 255, 255, 0.85);
    margin-top: 6%;
    border-radius: 20px;
    box-shadow: 0 16px 32px rgba(0, 0, 0, .12);
    max-width: 90%;
    max-height: 75%;
    overflow: auto;
    padding: 2%;
    position: relative;
}

/* Enter/leave animations */
.panel-enter-active .panel-content,
.panel-leave-active .panel-content {
    transition: all 0.3s ease;
}

.panel-enter-from .panel-content,
.panel-leave-to .panel-content {
    transform: scale(0.7);
    opacity: 0;
}

.panel-enter-active,
.panel-leave-active {
    transition: opacity 0.3s ease;
}
</style>