<script setup lang="ts">
const { title, hint, disabled, isGroup, labelComponent, labelComponentProps, labelAlignment } = defineProps<{
  title?: string;
  hint?: string;
  disabled?: boolean;
  isGroup?: boolean;
  labelComponent?: Object;
  labelComponentProps?: Object;
  labelAlignment?: 'right' | 'left';
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
  <label class="checkbox-wrapper" :class="disabled ? 'c-disabled' : 'c-regular'" :title="hint || title">
    <template v-if="labelAlignment === 'left'">
      <span @click="emit('titleClick')" whitespace-nowrap>
        <slot>
          <component v-if="labelComponent" :is="labelComponent" v-bind="labelComponentProps" />
          <span :class="isGroup ? 'cb-label-group' : 'cb-label'" v-else>{{ title }}</span>
        </slot>
      </span>
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
    </template>

    <template v-else>
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
    </template>
  </label>
</template>

<style lang="css" scoped>
.checkbox-wrapper {
  display: flex;
  align-items: center;
  gap: clamp(6px, 4%, 2rem);
}

.checkbox {
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 1.5vw;
  min-height: 1.5vw;
  margin-block: 0.5vh;
  background: white;
  border: 2px solid rgb(204, 204, 204);
  border-radius: 3px;
  cursor: pointer;
}
.cb-label {
  font-weight: 500;
  font-size: clamp(0.5rem, 2.5vh, 1.5rem);
}
.cb-label-group {
  --uno: 'c-active';
  font-weight: bold;
  font-size: clamp(0.5rem, 2.6vh, 1.5rem);
}
</style>
