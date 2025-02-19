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

export enum ComponentType {
  Tool = "Tool",
  ToolchainComponent = "ToolchainComponent",
  ToolchainProfile = "ToolchainProfile",
}
