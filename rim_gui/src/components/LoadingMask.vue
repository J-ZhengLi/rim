<script setup lang="ts">
import { ref } from 'vue';


const { text, finished } = defineProps({
    text: {
        type: String,
        default: 'loading...',
    },
    finished: {
        type: Boolean,
        default: false,
    }
});

// FIXME: type cannot be dynamically changed, maybe we should remove the original
// toast and make a new one when `finished`
const toastType = ref(finished ? 'success' : 'info');

</script>

<template>
    <div class="loading-mask">
        <base-toast
            :message="text"
            :duration="0"
            :is-loading="!finished"
            :type="toastType"
            decoration="spinner"
            position="top-right"
        />
    </div>
</template>

<style scoped>
.loading-mask {
    position: fixed;
    left: 0px;
    right: 0px;
    top: 0px;
    bottom: 0px;
    z-index: 998;
    background-color: rgba(73, 73, 73, 0.3);
}
.loading-toast-label {
    --uno: regular;
    font-size: 30px;
    z-index: 999;
}
</style>
