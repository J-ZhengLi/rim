import { ref, Ref, shallowRef } from 'vue';
import { KitItem } from './types/KitItem';
import { Component, ComponentType, componentUtils } from './types/Component';
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

  public componentsToUpdate(): CheckGroup<Component>[] {
    console.log("Installed components: ", this._installedKit.value?.components);
    console.log("Current components: ", this._current.value?.components);
    const checkItems: CheckGroupItem<Component>[] =
      this._current.value?.components
        .filter((item) => !componentUtils(item).isRestricted()) // ignore restricted components for now
        .map((item) => {
          const installedComps = this._installedKit.value?.components;

          // Note (J-ZhengLi): There was a bug where the `display-name`, which is what used to
          // represent rust toolchain got changed in a new toolkit, causing the app failed to
          // recognize the version of installed rust toolchain because the name not matches anymore.
          // Therefore here I directly use the installed toolchainVersion for `oldVer` if current
          // component item is the rust toolchain.
          const installedInfo = (() => {
            if (item.kind === ComponentType.ToolchainProfile) {
              const installedToolchain = installedComps?.find((c) => c.kind === ComponentType.ToolchainProfile);
              return {
                installed: installedToolchain !== undefined,
                version: installedToolchain?.version,
              };
            } else {
              const installedTool = installedComps?.find((c) => c.name === item.name);
              return {
                installed: installedTool !== undefined,
                version: installedTool?.version,
              };
            }
          })();

          let isVerDifferent = installedInfo.version && installedInfo.version !== item.version;
          let isRequiredButNotInstalled = item.required && !installedInfo.installed;

          let versionStr = isVerDifferent ? `(${installedInfo.version} -> ${item.version})` : ` (${item.version})`;

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
              oldVer: installedInfo.version,
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
    const installed = await invokeCommand(
      'get_installed_kit', { reload: true }
    ) as KitItem | undefined;
    if (installed) {
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
