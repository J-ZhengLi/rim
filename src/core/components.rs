use super::ToolkitManifestExt;
use crate::fingerprint::InstallationRecord;
use anyhow::Result;
use rim_common::types::{ToolInfo, ToolMap, ToolkitManifest};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    sync::atomic::{AtomicU32, Ordering},
};

static COMPONENTS_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, Hash, PartialEq, Eq)]
pub enum ComponentType {
    #[default]
    Tool,
    ToolchainComponent,
    ToolchainProfile,
}

impl ComponentType {
    /// Return `true` if this type is a toolchain component or a toolchain profile.
    pub fn is_from_toolchain(&self) -> bool {
        matches!(self, Self::ToolchainComponent | Self::ToolchainProfile)
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    pub id: u32,
    pub group_name: Option<String>,
    pub name: String,
    /// A name that used for display purpose, defaulting to `name`.
    pub display_name: String,
    pub version: Option<String>,
    pub desc: Option<String>,
    pub required: bool,
    pub optional: bool,
    pub tool_installer: Option<ToolInfo>,
    pub kind: ComponentType,
    /// Indicates whether this component was already installed or not.
    pub installed: bool,
}

impl Component {
    pub fn new(name: &str) -> Self {
        Component {
            id: COMPONENTS_COUNTER.fetch_add(1, Ordering::SeqCst),
            name: name.into(),
            display_name: name.into(),
            ..Default::default()
        }
    }

    /// Get a list of component names that are required by this component.
    pub fn dependencies(&self) -> &[String] {
        self.tool_installer
            .as_ref()
            .map(|info| info.dependencies())
            .unwrap_or_default()
    }

    /// Get a list of component names that are obsoleted (replaced) by this component.
    pub fn obsoletes(&self) -> &[String] {
        self.tool_installer
            .as_ref()
            .map(|info| info.obsoletes())
            .unwrap_or_default()
    }

    setter!(required(self.required, bool));
    setter!(optional(self.optional, bool));
    setter!(installed(self.installed, bool));
    setter!(set_kind(self.kind, ComponentType));
    setter!(with_group(self.group_name, group: Option<&str>) { group.map(ToOwned::to_owned) });
    setter!(with_tool_installer(self.tool_installer, installer: &ToolInfo) { Some(installer.clone()) });
    setter!(with_version(self.version, version: Option<&str>) { version.map(ToOwned::to_owned) });
    setter!(with_display_name(self.display_name, name: impl ToString) { name.to_string() });
    setter!(with_description(self.desc, desc: Option<&str>) { desc.map(ToOwned::to_owned) });
}

/// A Rust toolchain component, such as `rustc`, `cargo`, `rust-docs`
/// or even toolchain profile as as `minimal`, `default`.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ToolchainComponent {
    pub name: String,
    pub is_profile: bool,
}

impl ToolchainComponent {
    pub fn new<T: ToString>(name: T) -> Self {
        Self {
            name: name.to_string(),
            is_profile: false,
        }
    }
    setter!(is_profile(self.is_profile, bool));
}

/// Get a combined list of tools and toolchain components in Vec<[Component]> format,
/// whether it's installed or not.
///
/// A toolset manifest located under installation dir (`toolset-manifest.toml`)
/// will be loaded in order to retrieve component's full info.
///
/// # Panic
/// This should only be called in manager mode, otherwise it will panic.
pub(crate) fn all_components_from_installation(
    record: &InstallationRecord,
) -> Result<Vec<Component>> {
    let mut full_components =
        ToolkitManifest::load_from_install_dir()?.current_target_components(false)?;

    // components that are installed by rim previously.
    let installed_toolchain = record.installed_toolchain();
    let installed_tools: HashSet<&str> = record.installed_tools().collect();

    for comp in &mut full_components {
        match comp.kind {
            ComponentType::ToolchainComponent => {
                if let Some((tc, opt_comps)) = installed_toolchain {
                    comp.version = Some(tc.into());
                    comp.installed = opt_comps.iter().any(|c| c == &comp.name);
                }
            }
            ComponentType::ToolchainProfile => {
                comp.installed = installed_toolchain.is_some();
                comp.version = installed_toolchain.map(|(channel, _)| channel.to_string());
            }
            // third-party tools
            ComponentType::Tool => {
                if installed_tools.contains(comp.name.as_str()) {
                    comp.installed = true;
                    if let Some(ver) = record.get_tool_version(&comp.name) {
                        comp.version = Some(ver.into());
                    }
                }
            }
        }
    }

    Ok(full_components
        .into_iter()
        .filter(|c| c.installed)
        .collect())
}

pub fn component_list_to_tool_map(list: Vec<&Component>) -> ToolMap {
    list.iter()
        .filter_map(|c| {
            c.tool_installer
                .as_ref()
                .map(|tool_info| (c.name.clone(), tool_info.clone()))
        })
        .collect()
}
