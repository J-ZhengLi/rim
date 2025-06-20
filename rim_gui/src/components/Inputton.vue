<template>
    <div class="input-with-button">
        <input
            :value="modelValue"
            type="text"
            class="fused-input"
            :placeholder="placeholder"
        />
        <base-button theme="primary" class="fused-button" @click="handleButtonClick">
            {{ buttonLabel }}
        </base-button>
    </div>
</template>

<script setup lang="ts">
const props = defineProps<{
  modelValue: string;
  placeholder?: string;
  buttonLabel?: string;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void;
  (e: 'button-click', value: string): void;
}>();

function handleButtonClick() {
  emit('button-click', props.modelValue);
}
</script>

<style scoped>
.input-with-button {
    display: flex;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.1);
    border-radius: 10vw;
    overflow: hidden;
}

.fused-input {
    flex-grow: 1;
    padding: 0 2%;
    outline: none;
    border-radius: 1px 0 0 1px;
    background: rgba(255, 255, 255, .45);
    border: 2px solid transparent;
    backdrop-filter: url(#frosted);
    -webkit-backdrop-filter: blur(25px);
    font-size: clamp(100%, 2vh, 20px);
}

.fused-input:focus {
    background: rgba(255, 255, 255, .8);
}

.fused-button {
    padding: 0 3%;
    border-radius: 0 1px 1px 0;
}
</style>
