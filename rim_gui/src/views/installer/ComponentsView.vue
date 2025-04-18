<script setup lang="ts">
import { computed, onMounted, Ref, ref, watch } from 'vue';
import ScrollBox from '@/components/ScrollBox.vue';
import { componentUtils, installConf, invokeCommand } from '@/utils/index';
import type {
  CheckGroup,
  CheckGroupItem,
  CheckItem,
  Component,
  RestrictedComponent,
} from '@/utils/index';
import { useCustomRouter } from '@/router/index';
import CheckBoxGroup from '@/components/CheckBoxGroup.vue';

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
          required: item.required,
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
      const findItem = dependencies.find(([name, _]) => name === item.value.name);
      if (findItem) {
        item.checked = findItem[1];
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

  invokeCommand('get_restricted_components', { components: installConf.getCheckedComponents() }).then((res) => {
    const restricted = res as RestrictedComponent[];
    if (restricted.length > 0) {
      installConf.setRestrictedComponents(restricted);
      routerPush('/installer/confirm-package-sources');
    } else {
      routerPush('/installer/confirm');
    }
  })
}

onMounted(() => {
  groupComponents.value = installConf.getGroups();
});
</script>

<template>
  <div flex="~ col" w="full" h="full">
    <h4 ml="12px">安装选项</h4>
    <div flex="1 ~" p="12px" overflow="auto">
      <scroll-box overflow-auto p="4px" grow="1">
        <div p="t-8px l-8px">组件</div>
        <div ml="1.5rem">
          <base-check-box flex="~ items-center" v-model="checkedAllBundle" title="全选">
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
      </scroll-box>
      <scroll-box basis="200px" grow="4" ml="12px">
        <div>组件详细信息</div>
        <p font="b">{{ curCheckComponent?.value.displayName }}</p>
        <p>{{ curCheckComponent?.value.desc }}</p>
      </scroll-box>
    </div>

    <div basis="60px" flex="~ justify-end items-center">
      <base-button theme="primary" mr="12px" @click="routerBack">上一步</base-button>
      <base-button theme="primary" mr="12px" @click="handleNextClick">下一步</base-button>
    </div>
  </div>
</template>
