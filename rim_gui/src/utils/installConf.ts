import { ref, Ref } from 'vue';
import { isRecommended, toChecked, type Component, type RestrictedComponent } from './types/Component';
import { invokeCommand } from './invokeCommand';
import { CheckGroup, CheckItem } from './types/CheckBoxGroup';
import { AppInfo } from './types/AppInfo';

type EnforceableOption = [string, boolean];

interface BaseConfig {
  path: string;
  addToPath: boolean,
  insecure: boolean,
  rustupDistServer?: EnforceableOption,
  rustupUpdateRoot?: EnforceableOption,
  cargoRegistryName?: EnforceableOption,
  cargoRegistryValue?: EnforceableOption,
}

const defaultBaseConfig: BaseConfig = {
  path: '',
  addToPath: false,
  insecure: false,
};

class InstallConf {
  config: Ref<BaseConfig>;
  info: Ref<AppInfo | null> = ref(null);
  checkComponents: Ref<CheckItem<Component>[]>;
  version: Ref<string>;
  restrictedComponents: Ref<RestrictedComponent[]>;

  constructor() {
    this.config = ref(defaultBaseConfig);
    this.checkComponents = ref([]);
    this.version = ref('');
    this.restrictedComponents = ref([]);
  }

  /** The name and version of this application joined as a string. */
  async appNameWithShortVersion() {
    const shortenVersion = (ver: string) => {
      return ver.split(' ')[0];
    };

    if (this.info.value) {
      return this.info.value.version ? this.info.value.name + ' ' + shortenVersion(this.info.value.version) : this.info.value.name;
    }
    let info = await this.cacheAppInfo();
    return info.version ? info.name + ' ' + shortenVersion(info.version) : info.name;
  }

  async cacheAppInfo() {
    let info = await invokeCommand('app_info') as AppInfo;
    this.info.value = info;
    return info;
  }

  setPath(newPath: string) {
    this.config.value.path = newPath;
  }

  setComponents(newComponents: CheckItem<Component>[]) {
    const length = this.checkComponents.value.length;
    this.checkComponents.value.splice(0, length, ...newComponents);
  }

  setRestrictedComponents(comps: RestrictedComponent[]) {
    this.restrictedComponents.value = comps;
  }

  getGroups(): CheckGroup<Component>[] {
    const groups = this.checkComponents.value.reduce(
      (acc, item) => {
        const groupName = item.value.category;

        if (!acc[groupName]) {
          acc[groupName] = {
            label: groupName,
            items: [],
          };
        }

        acc[groupName].items.push({ ...item, focused: false });

        return acc;
      },
      {} as Record<string, CheckGroup<Component>>
    );
    return Object.values(groups);
  }

  getCheckedComponents(): Component[] {
    return this.checkComponents.value
      .filter((i) => i.checked) // 筛选选中组件
      .map((item: CheckItem<Component>) => {
        return item.value as Component;
      });
  }

  mapCheckedComponents(callback: (comp: CheckItem<Component>) => CheckItem<Component>) {
    this.checkComponents.value = this.checkComponents.value.map(callback)
  }

  getRestrictedComponents(): RestrictedComponent[] {
    return this.restrictedComponents.value;
  }

  async loadManifest(path?: string) {
    const ver = await invokeCommand("toolkit_version", { path: path });
    if (typeof ver === 'string') {
      this.version.value = ver;
    };

    // make sure the manifest is loaded before loading components
    // as it requires the manifest to be loaded.
    await this.loadDefaultConfig();
    await this.loadComponents();
  }

  async loadDefaultConfig() {
    const defaultConfig = await invokeCommand('default_configuration') as BaseConfig;
    this.config.value = defaultConfig;
  }

  async loadComponents() {
    const componentList = (await invokeCommand(
      'get_component_list'
    )) as Component[];
    if (Array.isArray(componentList)) {
      componentList.sort((a, b) => {
        // list pre-selected components at front.
        let aIsRecommended = isRecommended(a);
        let bIsRecommended = isRecommended(b);
        if (aIsRecommended && !bIsRecommended) {
          return -1;
        }
        if (!aIsRecommended && bIsRecommended) {
          return 1;
        }
        // 名称排序
        return a.displayName.localeCompare(b.displayName);
      });

      const newComponents = toChecked(componentList);
      this.setComponents(newComponents);
    }
  }
}

export const installConf = new InstallConf();
