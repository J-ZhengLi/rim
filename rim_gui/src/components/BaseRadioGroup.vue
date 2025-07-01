<template>
    <div class="radio-group" :class="alignmentClass" :style="{ gap: gap }">
        <label v-for="item in items" :key="item.value" class="radio-item"
            @mouseenter="activeHint = item.hint ? item.hint : null" @mouseleave="activeHint = null">
            <input type="radio" :value="item.value" :checked="modelValue === item.value"
                @change="$emit('update:modelValue', item.value)" />
            <span class="custom-radio"></span>
            <span class="label-text">{{ item.label }}</span>
        </label>

        <div v-if="activeHint" class="hint-bubble">
            {{ activeHint }}
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed, ref, type PropType } from 'vue';

export interface RadioItem {
    label: string;
    value: string | number;
    hint?: string;
}

const props = defineProps({
    items: {
        type: Array as PropType<RadioItem[]>,
        required: true,
        validator: (items: RadioItem[]) => items.length > 0
    },
    modelValue: {
        type: [String, Number] as PropType<string | number | null>,
        default: null
    },
    alignment: {
        type: String as PropType<'vertical' | 'horizontal'>,
        default: 'vertical',
        validator: (value: string) => ['vertical', 'horizontal'].includes(value)
    },
    gap: {
        type: String,
        default: 'auto'
    }
});
const emit = defineEmits(['update:modelValue']);

const activeHint = ref<string | null>(null);
const alignmentClass = computed(() => `align-${props.alignment}`);
</script>

<style scoped>
.radio-group {
    display: flex;
    position: relative;
}

.align-vertical {
    flex-direction: column;
}

.align-horizontal {
    flex-direction: row;
    align-items: center;
    flex-wrap: wrap;
}

.radio-item {
    display: flex;
    align-items: center;
    cursor: pointer;
    position: relative;
    padding: 1rem;
}

.label-text {
    margin-left: 1rem;
    font-size: clamp(0.5rem, 2.6vh, 1.5rem);
    color: var(--text-color, #333);
}

.hint-bubble {
    position: absolute;
    background-color: var(--hint-bg, #333);
    color: var(--hint-text, white);
    padding: 6px 12px;
    border-radius: 4px;
    font-size: 12px;
    white-space: nowrap;
    z-index: 100;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
    pointer-events: none;
}

.hint-bubble::before {
    content: '';
    position: absolute;
    top: -5px;
    border-width: 0 5px 5px 5px;
    border-style: solid;
    border-color: transparent transparent var(--hint-bg, #333) transparent;
}
</style>
