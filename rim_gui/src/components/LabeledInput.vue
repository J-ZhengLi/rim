<template>
    <div flex="~ col">
        <label :for="id" class="input-label">{{ label }}</label>
        <div class="input-wrapper">
            <input :disabled="disabled" :id="id" :value="modelValue" @input="handleInput" v-bind="$attrs" class="input-field" :class="{
                'bg-disabled-bg': disabled,
                'cursor-not-allowed': disabled,
            }" @mouseenter="showHint = true" @mouseleave="showHint = false" />
            <Transition name="fade">
                <div v-if="showHint && hint" class="hint-tooltip">
                    {{ hint }}
                </div>
            </Transition>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

defineProps<{
    label?: string;
    modelValue: string | null;
    hint?: string;
    disabled?: boolean;
}>();

const emit = defineEmits<{
    (e: 'update:modelValue', value: string | number | null): void
}>()

const id = `input-${Math.random().toString(36).slice(2, 11)}`
const showHint = ref(false)

const handleInput = (event: Event) => {
    emit('update:modelValue', (event.target as HTMLInputElement).value)
}
</script>

<style scoped>
.input-label {
    --uno: 'c-regular';
    margin-bottom: 0.5rem;
    font-weight: 500;
    font-size: clamp(0.5rem, 2.6vh, 1.5rem);
    flex-shrink: 0;
}

.input-wrapper {
    position: relative;
    flex-grow: 1;
    margin-bottom: 12px;
}

.input-field {
    width: 100%;
    background: rgba(255, 255, 255, .7);
    border: 1px solid transparent;
    backdrop-filter: url(#frosted);
    -webkit-backdrop-filter: blur(25px);
    font-size: clamp(100%, 2vh, 20px);
    padding: 0.3rem 1rem;
    box-sizing: border-box;
    border-radius: 20px;
    transition:
        border-color 0.2s,
        box-shadow 0.2s;
}

.input-field:focus {
    --uno: 'b-active';
    outline: none;
    box-shadow: 0 0 0 2px rgba(64, 158, 255, 0.2);
}

.hint-tooltip {
    position: absolute;
    bottom: calc(100% + 10px);
    left: 0;
    background-color: #333;
    color: white;
    padding: 0.5rem 1rem;
    border-radius: 4px;
    font-size: 13px;
    white-space: nowrap;
    z-index: 1000;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
    pointer-events: none;
}

.hint-tooltip::after {
    content: '';
    position: absolute;
    top: 100%;
    left: 12px;
    border-width: 6px;
    border-style: solid;
    border-color: #333 transparent transparent transparent;
}

.fade-enter-active,
.fade-leave-active {
    transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
    opacity: 0;
}
</style>