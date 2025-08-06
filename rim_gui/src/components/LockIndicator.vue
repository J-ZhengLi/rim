<template>
    <div class="lock-icon-wrapper" @mouseenter="showHint = true" @mouseleave="showHint = false">
        <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <g>
                <path
                    d="M13 14C13 13.4477 12.5523 13 12 13C11.4477 13 11 13.4477 11 14V16C11 16.5523 11.4477 17 12 17C12.5523 17 13 16.5523 13 16V14Z"
                    :fill="color" />
                <path fill-rule="evenodd" clip-rule="evenodd"
                    d="M7 8.12037C5.3161 8.53217 4 9.95979 4 11.7692V17.3077C4 19.973 6.31545 22 9 22H15C17.6846 22 20 19.973 20 17.3077V11.7692C20 9.95979 18.6839 8.53217 17 8.12037V7C17 4.23858 14.7614 2 12 2C9.23858 2 7 4.23858 7 7V8.12037ZM15 7V8H9V7C9 6.64936 9.06015 6.31278 9.17071 6C9.58254 4.83481 10.6938 4 12 4C13.3062 4 14.4175 4.83481 14.8293 6C14.9398 6.31278 15 6.64936 15 7ZM6 11.7692C6 10.866 6.81856 10 8 10H16C17.1814 10 18 10.866 18 11.7692V17.3077C18 18.7208 16.7337 20 15 20H9C7.26627 20 6 18.7208 6 17.3077V11.7692Z"
                    :fill="color" />
            </g>
        </svg>

        <div v-if="hint && showHint" class="tooltip">
            {{ hint }}
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
defineProps({
    color: {
        type: String,
        default: '#444',
    },
    hint: {
        type: String,
        default: '',
    },
});

const showHint = ref(false);
</script>

<style scoped>
.lock-icon-wrapper {
    position: relative;
    display: inline-block;
    width: 1.5rem;
    height: 1.5rem;
    margin-inline: 0.5rem;
}
</style>