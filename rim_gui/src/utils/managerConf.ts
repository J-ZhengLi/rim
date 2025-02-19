import { ref, Ref, shallowRef } from 'vue';
import { KitItem } from './types/KitItem';
import { Component } from './types/Component';
import { CheckGroup, CheckGroupItem } from './types/CheckBoxGroup';
import LabelComponent from '@/views/manager/components/Label.vue';
import { invokeCommand } from './invokeCommand';
import { AppInfo } from './types/AppInfo';

type Target = {
  operation: 'update' | 'uninstall';
  components: Component[];
};

class ManagerConf {
  path: Ref<string> = ref('');
  info: Ref<AppInfo | null> = ref(null);
  private _availableKits: Ref<KitItem[]> = ref([]);
  private _installedKit: Ref<KitItem | null> = ref(null);
  private _current: Ref<KitItem | null> = ref(null);
  private _target: Ref<Target> = ref({ operation: 'update', components: [] });
  private _isUninstallManager: Ref<boolean> = ref(false);

  constructor() { }

  /** The name of this application. */
  async appName() {
    if (this.info.value) {
      return this.info.value.name;
    }
    let info = await this.cacheAppInfo();
    return info.name;
  }

  /** The name and version of this application joined as a string. */
  async appNameWithVersion() {
    if (this.info.value) {
      return this.info.value.version ? this.info.value.name + ' ' + this.info.value.version : this.info.value.name;
    }
    let info = await this.cacheAppInfo();
    return info.version ? info.name + ' ' + info.version : info.name;
  }

  public getUninstallManager() {
    return this._isUninstallManager.value;
  }

  public getKits(): KitItem[] {
    return this._availableKits.value;
  }

  public getInstalled(): KitItem | null {
    return this._installedKit.value;
  }

  public getGroups(): CheckGroup<Component>[] {
    const checkItems: CheckGroupItem<Component>[] =
      this._current.value?.components.map((item) => {
        const installedItem = this._installedKit.value?.components.find(
          (c) => c.name === item.name
        );
        let installedVersion = installedItem?.version;
        let isVerDifferent = installedVersion !== undefined && installedVersion !== item.version;
        let isRequiredButNotInstalled = item.required && installedItem === undefined;

        let versionStr = isVerDifferent ? `(${installedVersion} -> ${item.version})` : ` (${item.version})`;

        return {
          label: `${item.displayName}${versionStr}`,
          checked: isVerDifferent || isRequiredButNotInstalled,
          required: item.required,
          disabled: false,

          focused: false,
          value: item,
          labelComponent: shallowRef(LabelComponent),
          labelComponentProps: {
            label: item.displayName,
            oldVer: installedVersion,
            newVer: item.version,
          },
        };
      }) || [];

    const groups = checkItems.reduce(
      (acc, item) => {
        const groupName = item.value.groupName
          ? item.value.groupName
          : 'Others'; // 确保 groupName 不为 null

        if (!acc[groupName]) {
          acc[groupName] = {
            label: groupName,
            items: [],
          };
        }

        acc[groupName].items.push({ ...item });

        return acc;
      },
      {} as Record<string, CheckGroup<Component>>
    );

    return Object.values(groups);
  }

  public getOperation() {
    return this._target.value.operation;
  }

  public getTargetComponents() {
    return this._target.value.components;
  }

  public setUninstallManager(value: boolean) {
    this._isUninstallManager.value = value;
  }

  public setKits(kits: KitItem[]): void {
    this._availableKits.value.splice(0, this._availableKits.value.length, ...kits);
  }

  public setInstalled(installed: KitItem): void {
    this._installedKit.value = installed;
  }

  public setCurrent(current: KitItem): void {
    this._current.value = current;
  }

  public setOperation(operation: Target['operation']): void {
    this._target.value.operation = operation;
  }
  public setComponents(components: Target['components']): void {
    this._target.value.components.splice(
      0,
      this._target.value.components.length,
      ...components
    );
  }

  async cacheAppInfo() {
    let info = await invokeCommand('app_info') as AppInfo;
    this.info.value = info;
    return info;
  }

  async loadConf() {
    let dir = await invokeCommand('get_install_dir');
    if (typeof dir === 'string' && dir.trim() !== '') {
      this.path.value = dir;
    }

    await this.reloadKits();
    // since this function is called immediately after app start, we call these functions
    // to check updates in background then ask user if they what to install it.
    await invokeCommand('check_updates_in_background');
  }

  async loadInstalledKit() {
    const tauriInstalled = (await invokeCommand(
      'get_installed_kit', { reload: true }
    )) as KitItem | undefined;
    if (tauriInstalled) {
      const installed = {
        ...tauriInstalled, components: tauriInstalled.components.filter((c) => c.installed).map((item) => {

          item.version = item.version || 'no version';
          return item as Component;
        })
      };
      this.setInstalled(installed);
      this.setCurrent(installed);
    }
  }

  async loadAvailableKits() {
    const availableKits = (await invokeCommand(
      'get_available_kits', { reload: true }
    )) as KitItem[];

    this.setKits(availableKits);
  }

  async reloadKits() {
    await this.loadInstalledKit()
    await this.loadAvailableKits()
  }
}

export const managerConf = new ManagerConf();
