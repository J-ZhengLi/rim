<script setup lang="ts">
const { title, hint, disabled, isGroup, labelComponent, labelComponentProps } = defineProps<{
  title?: string;
  hint?: string;
  disabled?: boolean;
  isGroup?: boolean;
  labelComponent?: Object;
  labelComponentProps?: Object;
}>();

const emit = defineEmits(['titleClick']);

const isChecked = defineModel<boolean>();

const toggleCheck = () => {
  if (disabled) {
    return;
  }

  isChecked.value = !isChecked.value;
};
</script>

<template>
  <label flex="inline items-center" :class="disabled ? 'c-disabled' : 'c-regular'" :title="hint || title" cursor-pointer>
    <span class="checkbox"
      :class="{
        'c-active': isGroup,
        'bg-active border-active': isChecked,
        'bg-disabled-bg': disabled,
        'hover:b-active': !disabled,
        'cursor-not-allowed': disabled,
      }" @click="toggleCheck">
      <slot name="icon">
        <i class="i-mdi:check" v-if="isChecked" c="active" />
      </slot>
    </span>
    <span @click="emit('titleClick')" whitespace-nowrap>
      <slot>
        <component v-if="labelComponent" :is="labelComponent" v-bind="labelComponentProps" />
        <span :class="isGroup ? 'cb-label-group' : 'cb-label'" v-else>{{ title }}</span>
      </slot>
    </span>
  </label>
</template>

<style lang="css" scoped>
.checkbox {
  /** flex="~ items-center justify-center" h="1rem" w="1rem" b="1px solid base" shrink="0" rounded="3px" bg="white" */
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 1rem;
  min-height: 1rem;
  background: white;
  border: 2px solid rgb(204, 204, 204);
  border-radius: 3px;
  margin-right: clamp(6px, 4%, 2rem);
}
.cb-label {
  font-weight: 500;
  font-size: clamp(0.5rem, 2.6vh, 1.5rem);
}
.cb-label-group {
  --uno: 'c-active';
  font-weight: bold;
  font-size: clamp(0.5rem, 2.7vh, 1.5rem);
}
</style>
