<script setup lang="ts">
import { computed, onMounted, onUpdated, Ref, ref, watch, nextTick } from 'vue';
import { componentUtils, managerConf } from '@/utils/index';
import type {
  CheckGroup,
  CheckGroupItem,
  Component,
} from '@/utils/index';
import { useCustomRouter } from '@/router/index';
import CheckBoxGroup from '@/components/CheckBoxGroup.vue';
import { message } from '@tauri-apps/api/dialog';

const { routerPush, routerBack } = useCustomRouter();
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

let backClicked = false;

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

function updateTargetComponents() {
  managerConf.setComponents(
    groupComponents.value.reduce((components, group) => {
      components.push(
        ...group.items.filter((i) => i.checked).map((item) => item.value)
      );
      return components;
    }, [] as Component[])
  );
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
      const findItem = dependencies.find(([name, _]) => name === item.value.name);
      if (findItem) {
        item.checked = findItem[1];
      }
    });
  });
}

function handleSelectAll() {
  const target = checkedAll.value;
  groupComponents.value.forEach((group) => {
    group.items.forEach((item) => {
      if (item.disabled) return;
      item.checked = !target;
    });
  });
}

function handleClickBack() {
  routerBack();
  nextTick(() => {
    backClicked = true;
  })
}

function handleClickNext() {
  let noSelection = groupComponents.value.every((item) =>
    item.items.every((i) => !i.checked)
  );
  if (noSelection) {
    message('请选择至少一个组件', { type: 'error' });
    return;
  }
  updateTargetComponents();
  routerPush('/manager/confirm');
}

function refreshComponents() {
  groupComponents.value = managerConf.getCheckGroups();
  updateTargetComponents();
}

onMounted(() => refreshComponents());
onUpdated(() => {
  // only update components list if "back" was clicked,
  // the only downside of this is it will refresh component selections once the
  // user have clicked "back" but then select the same toolkit again,
  // but it might not be that important to keep the same selections.
  if (backClicked) {
    groupComponents.value = managerConf.getCheckGroups();
    backClicked = false;
  }
});
</script>

<template>
  <div flex="~ col" w="full" h="full">
    <span class="info-label">{{ $t('select_components_to_install') }}</span>
    <p class="sub-info-label">{{ $t('select_components_gui_hint') }}</p>
    <split-box flex="1 ~" mb="10vh" mx="1vw" leftWidth="45%">
      <template #left>
        <b>{{ $t('components') }}</b>
        <div p="t-8px" flex="~ items-center wrap" gap="3">
          <span>
            <base-tag size="small" w="1em" h="1.5em" m="r-2px b-4px"></base-tag>
            {{ $t('current_version') }}
          </span>
          <span>
            <base-tag type="success" size="small" w="1em" h="1.5em" m="r-2px b-4px"></base-tag>
            {{ $t('new_version') }}
          </span>
          <span>
            <base-tag type="warning" size="small" w="1em" h="1.5em" m="r-2px b-4px"></base-tag>
            {{ $t('old_version') }}
          </span>
        </div>
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
    <page-nav-buttons :backLabel="$t('back')" :nextLabel="$t('next')" @back-clicked="handleClickBack" @next-clicked="handleClickNext" />
  </div>
</template>
