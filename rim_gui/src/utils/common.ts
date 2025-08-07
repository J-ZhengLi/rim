import { installConf } from "./installConf";
import { invokeCommand } from "./invokeCommand";
import { AppInfo } from "./types/AppInfo";
import { RestrictedComponent } from "./types/Component";

export type EnforceableOption = [string, boolean];

export interface BaseConfig {
  path: string;
  addToPath: boolean,
  insecure: boolean,
  rustupDistServer?: EnforceableOption,
  rustupUpdateRoot?: EnforceableOption,
  cargoRegistryName?: EnforceableOption,
  cargoRegistryValue?: EnforceableOption,
}

export const defaultBaseConfig: BaseConfig = {
  path: '',
  addToPath: false,
  insecure: false,
};

/**
 * Handle the restricted components before installation,
 * as some components might need another package source.
 * 
 * @param onDefault The default callback where there aren't any restricted components.
 * @param onRestricted Callback when restricted components detected in `installConf`.
 */
export function handleRestrictedComponents(onDefault: () => void, onRestricted: () => void) {
  invokeCommand('get_restricted_components', { components: installConf.getCheckedComponents() }).then((res) => {
    const restricted = res as RestrictedComponent[];
    if (restricted.length > 0) {
      installConf.setRestrictedComponents(restricted);
      onRestricted();
    } else {
      onDefault();
    }
  });
}

/** The name and version of this application. */
export async function getAppNameWithVersion(): Promise<[string, string]> {
  const shortenVersion = (ver: string) => {
    return ver.split(' ')[0];
  };
  const info = await invokeCommand('app_info') as AppInfo;
  console.log(info);
  return [info.name, shortenVersion(info.version)];
}
