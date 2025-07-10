<script setup lang="ts">
import { computed, onMounted, Ref, ref, watch } from 'vue';
import { componentUtils, installConf } from '@/utils/index';
import type {
  CheckGroup,
  CheckGroupItem,
  CheckItem,
  Component,
} from '@/utils/index';
import { useCustomRouter } from '@/router/index';
import CheckBoxGroup from '@/components/CheckBoxGroup.vue';
import { handleRestrictedComponents } from '@/utils/common';

const { routerBack, routerPush } = useCustomRouter();
const selectComponentId = ref(0);

const groupComponents: Ref<CheckGroup<Component>[]> = ref([]);
const checkedAllBundle = ref(false);
const checkedAll = computed(() => {
  return groupComponents.value.every((item) =>
    item.items.every((i) => i.checked)
  );
});
const checkedEmpty = computed(() => {
  return groupComponents.value.every((item) =>
    item.items.every((i) => !i.checked)
  );
});

watch(checkedAll, (val) => {
  checkedAllBundle.value = val;
});

const curCheckComponent = computed(() => {
  for (const group of groupComponents.value) {
    for (const item of group.items) {
      if (item.focused) {
        return item;
      }
    }
  }
  return null;
});

function updateInstallConf() {
  const comps = groupComponents.value.reduce((components, group) => {
    components.push(
      ...group.items.map((item) => {
        return {
          label: item.label,
          checked: item.checked,
          disabled: item.disabled,
          value: { ...item.value },
        };
      })
    );
    return components;
  }, [] as CheckItem<Component>[]);
  installConf.setComponents(comps);
}

function handleComponentsClick(checkItem: CheckGroupItem<Component>) {
  selectComponentId.value = checkItem.value.id;
  groupComponents.value.forEach((group) => {
    group.items.forEach((item) => {
      if (item.value.id === checkItem.value.id) {
        item.focused = true;
      } else {
        item.focused = false;
      }
    });
  });
}

// FIXME: this function somehow gets called with each component title clicks,
// and the body of it is not efficient at all.
function handleComponentsChange(items: CheckGroupItem<Component>[]) {
  let dependencies: [string, boolean][] = [];

  groupComponents.value.forEach((group) => {
    group.items.forEach((item) => {
      const findItem = items.find((i) => i.value.id === item.value.id);
      if (findItem) {
        item.checked = findItem.checked;
        dependencies = dependencies.concat(componentUtils(item.value).requires().map(name => [name, findItem.checked]));
      }
    });
  });

  // add dependencies
  groupComponents.value.forEach((group) => {
    group.items.forEach((item) => {
      if (!item.value.installed) {
        const findItem = dependencies.find(([name, _]) => name === item.value.name);
        if (findItem) {
          item.checked = findItem[1];
        }
      }
    });
  });
}

function handleSelectAll() {
  const target = !checkedAll.value;
  groupComponents.value.forEach((group) => {
    group.items.forEach((item) => {
      if (item.disabled) return;
      item.checked = target;
    });
  });
}

function handleNextClick() {
  updateInstallConf();
  handleRestrictedComponents(
    () => routerPush('/installer/confirmation'),
    () => routerPush('/installer/customize_package_sources'),
  );
}

onMounted(() => {
  groupComponents.value = installConf.getGroups();
});
</script>

<template>
  <div flex="~ col" w="full" h="full">
    <span class="info-label">{{ $t('select_components_to_install') }}</span>
    <p class="sub-info-label">{{ $t('select_components_gui_hint') }}</p>
    <split-box flex="1 ~" mb="10vh" mx="1vw">
      <template #left>
        <b>{{ $t('components') }}</b>
        <div mt="0.5rem">
          <base-check-box flex="~ items-center" v-model="checkedAllBundle" :title="$t('select_all')">
            <template #icon>
              <span flex="~ items-center justify-center" w="full" h="full" @click="handleSelectAll">
                <i class="i-mdi:check" v-show="checkedAll" c="active" />
                <i class="i-mdi:minus" v-show="!checkedAll && !checkedEmpty" c="active" />
              </span>
            </template>
          </base-check-box>
        </div>

        <check-box-group v-for="group of groupComponents" :key="group.label" :group="group" expand
          @itemClick="handleComponentsClick" @change="handleComponentsChange" />
      </template>

      <template #right>
        <b>{{ $t('description') }}</b>
        <p mr="1.5rem">{{ curCheckComponent?.value.desc }}</p>
        <div>
          <b>{{ $t('type') }}</b>
          <p>{{ curCheckComponent?.value.kindDesc.name }}</p>
        </div>
        <div v-if="curCheckComponent?.value.kindDesc.help">
          <b>{{ $t('type_desc') }}</b>
          <p mr="1.5rem">{{ curCheckComponent?.value.kindDesc.help }}</p>
        </div>
      </template>
    </split-box>

    <page-nav-buttons :backLabel="$t('back')" :nextLabel="$t('next')" @back-clicked="routerBack" @next-clicked="handleNextClick" />
  </div>
</template>
