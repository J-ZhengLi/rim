import { CheckItem } from "./CheckBoxGroup";

export class ComponentHelper {
  component: Component;

  constructor(component: Component) {
    this.component = component;
  }

  getToolInfo(): ToolInfoDetails | undefined {
    if (this.component.toolInstaller && typeof this.component.toolInstaller !== 'string') {
      return this.component.toolInstaller
    }
  }

  requires(): string[] {
    const info = this.getToolInfo();
    if (info) {
      return info.requires;
    }
    return [];
  }

  obsoletes(): string[] {
    const info = this.getToolInfo();
    if (info) {
      return info.obsoletes;
    }
    return [];
  }
}

// Reflecting the `Component` type in `src/core/components.rs`
export interface Component {
  id: number;
  groupName?: string;
  name: string;
  displayName: string;
  version?: string;
  desc?: string;
  required: boolean;
  optional: boolean;
  toolInstaller?: string | ToolInfoDetails;
  kind: ComponentType;
  installed: boolean;
}

type ToolInfoDetails =
  | RestrictedTool
  | GitTool
  | UrlTool
  | PathTool
  | VersionTool

// Reflecting the `ToolInfoDetails` type in `rim_common/src/types/tool_info.rs`
interface BaseToolInfoDetails {
  required: boolean;
  optional: boolean;
  requires: string[];
  obsoletes: string[];
  conflicts: string[];
}

interface RestrictedTool extends BaseToolInfoDetails {
  restricted: boolean;
  default?: string;
  source?: string;
  version?: string;
}

interface GitTool extends BaseToolInfoDetails {
  git: string;
  branch?: string;
  tag?: string;
  rev?: string;
}

interface UrlTool extends BaseToolInfoDetails {
  version?: string;
  url: string;
  filename?: string;
}

interface PathTool extends BaseToolInfoDetails {
  version?: string;
  path: string;
}

interface VersionTool extends BaseToolInfoDetails {
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
