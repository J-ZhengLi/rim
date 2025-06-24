<template>
    <base-card overflow-auto>
        <div class="resizable-container" ref="container">
            <div class="panel" :style="{ width: leftWidth + 'px' }" ref="leftPanel">
                <slot name="left"></slot>
            </div>
            <div class="divider" @mousedown="startDrag"></div>
            <div class="panel right-panel" ref="rightPanel">
                <slot name="right"></slot>
            </div>
        </div>
    </base-card>
</template>

<script setup lang="ts">
import { ref, onUnmounted } from 'vue'

const container = ref<HTMLElement | null>(null)
const leftPanel = ref<HTMLElement | null>(null)
const leftWidth = ref(300)
const minWidth = 100
const isDragging = ref(false)

const startDrag = (e: MouseEvent) => {
    isDragging.value = true
    document.addEventListener('mousemove', onDrag)
    document.addEventListener('mouseup', stopDrag)
    e.preventDefault()
}

const onDrag = (e: MouseEvent) => {
    if (!isDragging.value || !container.value) return

    const containerRect = container.value.getBoundingClientRect()
    const newLeftWidth = Math.max(
        minWidth,
        Math.min(
            e.clientX - containerRect.left,
            containerRect.width - minWidth
        )
    )

    leftWidth.value = newLeftWidth
}

const stopDrag = () => {
    isDragging.value = false
    document.removeEventListener('mousemove', onDrag)
    document.removeEventListener('mouseup', stopDrag)
}

// Cleanup event listeners
onUnmounted(stopDrag)
</script>

<style scoped>
.resizable-container {
    display: flex;
    width: 100%;
    height: 100%;
}

.panel {
    height: 100%;
    overflow: auto;
    box-sizing: border-box;
}

.right-panel {
    flex: 1;
    padding-left: 2vw;
}

.divider {
    width: 5px;
    background-color: rgba(255, 255, 255, 0.7);
    
    cursor: col-resize;
    position: relative;
    transition: background-color 0.2s;
}

.divider:hover,
.divider:active {
    background-color: #aaa;
}

.divider::before {
    content: '';
    position: absolute;
    top: 50%;
    left: 1px;
    right: 1px;
    height: 30px;
    margin-top: -15px;
    --uno: 'bg-primary'
    border-radius: 2px;
}
</style>
