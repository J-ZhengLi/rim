<script setup lang="ts">
import { invokeCommand, KitItem, managerConf, ManagerOperation } from '@/utils/index';
import { useCustomRouter } from '@/router/index';

const { routerPush } = useCustomRouter();

const props = defineProps<{
  kit: KitItem;
  installed: boolean;
}>();

const handleUpdate = () => {
  managerConf.setOperation(ManagerOperation.Modify);
  routerPush('/manager/change');
};

const handleUninstall = () => {
  managerConf.setOperation(ManagerOperation.UninstallToolkit);
  routerPush('/manager/uninstall');
};

const handleInstall = () => {
  invokeCommand('get_toolkit_from_url', {
    url: props.kit.manifestURL as string,
  }).then((toolkit) => {
    const kit = toolkit as KitItem;
    props.kit.components = kit.components
    managerConf.setCurrent(kit);
    managerConf.setOperation(ManagerOperation.Update);
    routerPush('/manager/change');
  });
};
</script>
<template>
  <div shadow flex="~ justify-between" p="8px" mx="12px" rounded="4px" b="1px solid light">
    <div>
      <div>
        <p flex="~ items-center">
          <img src="/favicon.ico" h="2rem" />
          <span ml="1rem">{{ props.kit.name }}</span>
        </p>
        <p ml="3rem">{{ props.kit.version }}</p>
        <p ml="3rem">{{ props.kit.desc }}</p>
        <!-- TODO: There should a button labeled as "Changelog" that shows `kit.info` in a pop-up when clicked -->
        <!-- <a m="l-3rem t-0.5rem">{{ props.kit.info }}</a> -->
      </div>
    </div>
    <div v-if="props.installed" flex="~ col justify-around">
      <base-button p="y-2px x-24px" theme="primary" @click="handleUpdate" hidden>更改</base-button>
      <base-button p="y-2px x-24px" @click="handleUninstall">卸载</base-button>
    </div>
    <div v-else flex="~ col justify-around">
      <base-button p="y-2px x-24px" theme="primary" @click="handleInstall">安装</base-button>
    </div>
  </div>
</template>

<style scoped>
p {
  margin-top: 0.5rem;
  margin-bottom: 0;
}
</style>
