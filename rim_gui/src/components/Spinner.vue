<template>
    <div flex="~ justify-center items-center">
        <div class="spinner" :class="themeClass" :style="{
            width: size,
            height: size,
        }"></div>
    </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps({
    size: {
        type: String,
        default: '16px',
    },
    color: {
        type: String,
        default: 'white',
    }
});

const themeClass = computed(() => {
    switch (props.color) {
        case 'black':
            return "spinner-black";
        case 'blue':
            return "spinner-blue";
        default:
            return "spinner-white";
    }
});
</script>

<style lang="css" scoped>
.spinner {
    display: inline-block;
    animation: spin 1s linear infinite;
    border-style: solid;
    border-radius: 50%;
}

.spinner-white {
    border: 3px solid rgba(255, 255, 255, 0.3);
    border-top: 3px solid #fff;
}

.spinner-black {
    border: 3px solid rgba(0, 0, 0, 0.2);
    border-top: 3px solid #222;
}

.spinner-blue {
    border: 3px solid #a0dcff3a;
    border-top: 3px solid #5b98d8;
}

@keyframes spin {
    0% {
        transform: rotate(0deg);
    }

    100% {
        transform: rotate(360deg);
    }
}
</style>
