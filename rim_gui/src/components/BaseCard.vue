<template>
    <div class="glass" :class="{'interactive': props.interactive}">
        <slot></slot>
    </div>
</template>

<script setup lang="ts">
const props = defineProps({
    interactive: {
        type: Boolean,
        default: false
    }
});
</script>

<style scoped>
.glass {
    padding: 2%;
    border-radius: 20px;
    border: 2px solid transparent;
    box-shadow: 0 0 0 2px rgba(255, 255, 255, .6), 0 12px 16px rgba(0, 0, 0, .12);
    backdrop-filter: blur(25px);
    outline: 0;
    transition: background-color 0.5s ease-out;
}

.glass:hover {
    background-color: rgba(255, 255, 255, 0.45);
}

.interactive {
    cursor: pointer;
}
.interactive:hover {
    box-shadow: 0 0 0 2px rgba(91, 155, 213, .12), 0 12px 16px rgba(91, 155, 213, .8);
}
</style>
