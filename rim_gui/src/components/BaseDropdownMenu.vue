<script setup lang="ts">
import { computed, PropType, ref } from 'vue';

export interface DropdownItem {
    value: string;
    label: string;
}

const props = defineProps({
    items: {
        type: Array as PropType<DropdownItem[]>,
        required: true,
        default: () => []
    },
    modelValue: {
        type: String as PropType<string | null>,
        default: null
    },
    placeholder: {
        type: String,
        default: ''
    },
    width: {
        type: String,
        default: 'auto'
    },
    height: {
        type: String,
        default: 'auto'
    },
    default: {
        type: Object,
        default: null
    }
});

const emit = defineEmits(['update:modelValue']);

const isOpen = ref(false);
const menuRef = ref<HTMLElement | null>(null);

const selectedItem = computed(() =>
    props.items.find(item => item.value === props.modelValue)
);


function toggleDropdown() {
    isOpen.value = !isOpen.value;
}

function selectItem(value: string | number) {
    emit('update:modelValue', value);
    isOpen.value = false;
}

function closeOnClickOutside(event: MouseEvent) {
    if (menuRef.value && !menuRef.value.contains(event.target as Node)) {
        isOpen.value = false;
    }
}

// Close dropdown when clicking outside
document.addEventListener('click', closeOnClickOutside);
</script>

<template>
    <div class="dropdown" ref="menuRef">
        <div class="dropdown-toggle" :class="{ 'dropdown-open': isOpen }" @click.stop="toggleDropdown" :style="{
            width: width,
            height: height,
        }">
            <span>{{ selectedItem ? selectedItem.label : placeholder }}</span>
            <svg class="dropdown-arrow" viewBox="0 0 24 24" :class="{ rotated: isOpen }">
                <path d="M7 10l5 5 5-5z" />
            </svg>
        </div>

        <Transition name="dropdown">
            <ul v-show="isOpen" class="dropdown-menu">
                <li v-for="item in items" :key="item.value" class="dropdown-item"
                    :class="{ selected: modelValue === item.value }" @click="selectItem(item.value)">
                    {{ item.label }}
                </li>
            </ul>
        </Transition>
    </div>
</template>

<style scoped>
.dropdown {
    position: relative;
    margin: 1rem;
    height: 100%;
    width: 100%;
}

.dropdown-toggle {
    display: flex;
    justify-content: space-between;
    align-items: center;
    cursor: pointer;
    transition: border-color 0.3s;
    background: rgba(255, 255, 255, .7);
    border: 1px solid rgba(0, 0, 0, .1);
    box-shadow: 0 8px 16px rgba(0, 0, 0, 0.1);
    backdrop-filter: url(#frosted);
    -webkit-backdrop-filter: blur(25px);
    font-size: clamp(100%, 2vh, 20px);
    padding: 0.3rem 1rem;
    box-sizing: border-box;
    border-radius: 20px;
}

.dropdown-toggle:hover {
    --uno: 'b-base';
}

.dropdown-toggle.dropdown-open {
    --uno: 'b-active';
}

.dropdown-arrow {
    width: 16px;
    height: 16px;
    transition: transform 0.3s;
}

.dropdown-arrow.rotated {
    transform: rotate(180deg);
}

.dropdown-menu {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin-top: 4px;
    padding: 0;
    border: 1px solid #ddd;
    border-radius: 10px;
    background-color: white;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    list-style: none;
    z-index: 100;
    max-height: 200px;
    overflow-y: auto;
}

.dropdown-item {
    padding: 8px 12px;
    cursor: pointer;
    transition: background-color 0.2s;
}

.dropdown-item:hover {
    background-color: #f1f1f1;
}

.dropdown-item.selected {
    background-color: #eef5ff;
    --uno: 'c-active'
}

/* Transition effects */
.dropdown-enter-active,
.dropdown-leave-active {
    transition: all 0.3s ease;
    transform-origin: top center;
}

.dropdown-enter-from,
.dropdown-leave-to {
    opacity: 0;
    transform: scaleY(0.9);
}

.dropdown-enter-to,
.dropdown-leave-from {
    opacity: 1;
    transform: scaleY(1);
}
</style>
