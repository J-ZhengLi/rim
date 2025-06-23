import { CheckItem } from "./CheckBoxGroup";

export function componentUtils(component: Component) {
  return {
    getToolInfo(): ToolInfoDetails | undefined {
      if (component.toolInstaller && typeof component.toolInstaller !== 'string') {
        return component.toolInstaller
      }
    },
  
    requires(): string[] {
      const req = this.getToolInfo()?.requires;
      return req ?? [];
    },
  
    obsoletes(): string[] {
      const obs = this.getToolInfo()?.obsoletes;
      return obs ?? [];
    },

    isRestricted(): boolean {
      const info = this.getToolInfo();
      if (info && 'restricted' in info) {
        return info.restricted;
      }
      return false;
    }
  }
}

// Reflecting the `Component` type in `src/core/components.rs`
export interface Component {
  id: number;
  category: string;
  name: string;
  displayName: string;
  version?: string;
  desc?: string;
  required: boolean;
  optional: boolean;
  toolInstaller?: string | ToolInfoDetails;
  kind: ComponentType;
  kindDesc: ComponentTypeDesc;
  installed: boolean;
}

export type ToolInfoDetails =
  | RestrictedTool
  | GitTool
  | UrlTool
  | PathTool
  | VersionTool

// Reflecting the `ToolInfoDetails` type in `rim_common/src/types/tool_info.rs`
export interface BaseToolInfoDetails {
  required: boolean;
  optional: boolean;
  requires?: string[];
  obsoletes?: string[];
  conflicts?: string[];
}

export interface RestrictedTool extends BaseToolInfoDetails {
  restricted: boolean;
  default?: string;
  source?: string;
  version?: string;
}

export interface GitTool extends BaseToolInfoDetails {
  git: string;
  branch?: string;
  tag?: string;
  rev?: string;
}

export interface UrlTool extends BaseToolInfoDetails {
  version?: string;
  url: string;
  filename?: string;
}

export interface PathTool extends BaseToolInfoDetails {
  version?: string;
  path: string;
}

export interface VersionTool extends BaseToolInfoDetails {
  version: string,
}

export interface RestrictedComponent {
  name: string;
  label: string;
  source?: string;
  default?: string;
}

export enum ComponentType {
  Tool = "Tool",
  ToolchainComponent = "ToolchainComponent",
  ToolchainProfile = "ToolchainProfile",
}

export interface ComponentTypeDesc {
  name: string,
  help?: string,
}

export function toChecked(components: Component[]): CheckItem<Component>[] {
  return components.map(
    (item) => {
      return {
        label: `${item.displayName}${item.installed ? ' (installed)' : item.required ? ' (required)' : ''}`,
        checked: !item.installed && (item.required || !item.optional),
        required: item.required,
        disabled: item.installed ? false : item.required,
        value: item,
      } as CheckItem<Component>;
    }
  );
}
