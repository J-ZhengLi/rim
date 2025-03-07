use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::{
    fingerprint::InstallationRecord,
    setter,
    toolset_manifest::{ToolInfo, ToolMap, ToolsetManifest},
};

static COMPONENTS_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum ComponentType {
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    pub id: u32,
    pub group_name: Option<String>,
    pub name: String,
    /// A name that used for display purpose, defaulting to `name`.
    pub display_name: String,
    pub version: Option<String>,
    pub desc: String,
    pub required: bool,
    pub optional: bool,
    pub tool_installer: Option<ToolInfo>,
    pub kind: ComponentType,
    /// Indicates whether this component was already installed or not.
    pub installed: bool,
}

impl Component {
    #[must_use]
    pub fn new(name: &str, desc: &str) -> Self {
        let comp = Component {
            id: COMPONENTS_COUNTER.load(Ordering::Relaxed),
            group_name: None,
            name: name.into(),
            display_name: name.into(),
            version: None,
            desc: desc.into(),
            required: false,
            optional: false,
            tool_installer: None,
            kind: ComponentType::Tool,
            installed: false,
        };
        COMPONENTS_COUNTER.fetch_add(1, Ordering::SeqCst);

        comp
    }

    setter!(required(self.required, bool));
    setter!(optional(self.optional, bool));
    setter!(installed(self.installed, bool));
    setter!(set_kind(self.kind, ComponentType));
    setter!(with_group(self.group_name, group: Option<&str>) { group.map(ToOwned::to_owned) });
    setter!(with_tool_installer(self.tool_installer, installer: &ToolInfo) { Some(installer.clone()) });
    setter!(with_version(self.version, version: Option<&str>) { version.map(ToOwned::to_owned) });
    setter!(with_display_name(self.display_name, name: impl ToString) { name.to_string() });
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
        ToolsetManifest::load_from_install_dir()?.current_target_components(false)?;

    // components that are installed by rim previously.
    let installed_toolchain = record.installed_toolchain();
    let installed_tools: HashSet<&str> = record.installed_tools().collect();

    for comp in &mut full_components {
        if comp.kind.is_from_toolchain() {
            if let Some((tc, opt_comps)) = installed_toolchain {
                comp.version = Some(tc.into());
                comp.installed = opt_comps.iter().any(|c| c == &comp.name);
            }
            continue;
        }
        // third-party tools
        if installed_tools.contains(comp.name.as_str()) {
            comp.installed = true;
            if let Some(ver) = record.get_tool_version(&comp.name) {
                comp.version = Some(ver.into());
            }
        }
    }

    Ok(full_components)
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
