import { Component } from './Component';

interface BaseKitItem {
  /**
   * The version number
   */
  version: string;

  /**
   * The Kit name
   */
  name: string;

  /**
   * The version description
   */
  desc: string;

  /**
   * The version release notes
   */
  info: string;

  manifestURL: string;
}

export interface KitItem extends BaseKitItem {
  components: Component[];
}

/**
 * The type of distributing toolkit
 */
export enum ToolkitKind {
  /**
   * Built-in toolkit definition. (`BuiltIn` might be a bad name, subject to change in the future)
   * The name of this toolkit is built-in by this program, and was defined in `configuration.toml`
   */
  BuiltIn = "BuiltIn",
  /**
   * Customize toolkit. Which the toolchain server, registry index, etc,
   * can be customized by users.
   */
  Native = "Native",
}
