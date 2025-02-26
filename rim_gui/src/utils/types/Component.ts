import { CheckItem } from "./CheckBoxGroup";

export interface Component {
  id: number;
  name: string;
  displayName: string;
  version?: string;
  required: boolean;
  optional: boolean;
  installed: boolean;
  desc: string;
  groupName: string | null;
  kind: ComponentType;
  toolInstaller?: {
    required: boolean;
    optional: boolean;
    path?: string;
  };
}

export interface RestrictedComponent {
  name: string,
  label: string,
  source?: string,
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
