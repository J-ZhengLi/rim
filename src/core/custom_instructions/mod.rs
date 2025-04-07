use std::env::consts::EXE_SUFFIX;
use std::{collections::HashMap, sync::LazyLock};

use rim_common::utils;

macro_rules! declare_instructions {
    ($($name:ident),+) => {
        $(pub(crate) mod $name;)*
        pub(crate) static SUPPORTED_TOOLS: &[&str] = &[$(stringify!($name)),+];

        pub(crate) fn install(tool: &str, path: &std::path::Path, config: &super::install::InstallConfiguration) -> anyhow::Result<Vec<std::path::PathBuf>> {
            match tool.replace('-', "_").as_str() {
                $(
                    stringify!($name) => $name::install(path, config),
                )*
                _ => anyhow::bail!("no custom install instruction for '{tool}'")
            }
        }

        pub(crate) fn uninstall(tool: &str, config: &super::uninstall::UninstallConfiguration) -> anyhow::Result<()> {
            match tool.replace('-', "_").as_str() {
                $(
                    stringify!($name) => $name::uninstall(config),
                )*
                _ => anyhow::bail!("no custom uninstall instruction for '{tool}'")
            }
        }

        fn supported_tool_is_installed(tool: &str) -> bool {
            match tool.replace('-', "_").as_str() {
                $(
                    stringify!($name) => $name::already_installed(),
                )*
                // Is not supported, assume not installed for now
                _ => false
            }
        }
    };
}

#[cfg(windows)]
declare_instructions!(buildtools, vscode, vscodium, codearts_rust);
#[cfg(not(windows))]
declare_instructions!(vscode, vscodium, codearts_rust);

pub(crate) fn is_supported(name: &str) -> bool {
    SUPPORTED_TOOLS.contains(&name.replace('-', "_").as_str())
}

/// This is a map with tool's name and a list of programs to check for existence.
/// Since the list to check is highly rely on tool's name, let's calling it `semi-supported` for now.
static SEMI_SUPPORTED_TOOLS: LazyLock<HashMap<&str, Vec<String>>> = LazyLock::new(|| {
    HashMap::from([
        (
            "mingw64",
            vec![format!("gcc{EXE_SUFFIX}"), format!("ld{EXE_SUFFIX}")],
        ),
        ("codearts-rust", vec!["codearts-rust".into()]),
    ])
});

/// Checking if a certain tool is installed by:
///
/// 1. If it has it's on module, it should be determined there, see list: [`SUPPORTED_TOOLS`].
/// 2. Looking up the same name in path.
/// 3. Looking up a pre-defined list related to the given tool, to see if
///    those are all in the path.
pub(crate) fn is_installed(name: &str) -> bool {
    if supported_tool_is_installed(name) || utils::cmd_exist(exe!(name)) {
        return true;
    }

    let programs = SEMI_SUPPORTED_TOOLS.get(name);
    if let Some(list) = programs {
        list.iter().all(utils::cmd_exist)
    } else {
        // Still have no idea, assuming not installed
        false
    }
}
