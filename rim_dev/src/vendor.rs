use crate::{
    common::{download, ensure_dir, ensure_parent_dir, resources_dir},
    toolkits_parser::{Component, GlobalConfig, Toolkits},
};
use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use std::{fs, path::Path};
use toml::{map::Map, Value};

const TOOLS_DIRNAME: &str = "tools";
const TOOLCHAIN_DIRNAME: &str = "toolchain";

pub(super) const VENDOR_HELP: &str = r#"
Split `toolkits.toml` and download packages specified in it for offline packaging

Usage: cargo dev vendor [OPTIONS]

Options:
    -n, --name      Name of the toolkit to vendor for, if not provided, all toolkits will be vendored
    -t, --target    Specify a target when downloading packages, defaulting to current running target
    -a, --all-targets
                    Download packages for all supporting targets
        --download-only
                    Do not update toolkit-manifests, just download packages
        --split-only
                    Update toolkit-manifests by spliting the `toolkits.toml` under resources folder, but don't download packages.
                    Note that spliting will generate offline toolset-manifest as well,
                    which might not work properly if the packages are not downloaded.
    -h, -help       Print this help message
"#;

const TOOLSET_MANIFEST_HEADER: &str = "
# This file was automatically generated.
# 此文件是自动生成的.
";

#[derive(Debug, Default, Clone, Copy)]
pub(super) enum VendorMode {
    /// Default behavior, split toolkit manifests and download packages
    #[default]
    Regular,
    /// Only download packages, don't modify toolkit manifests
    DownloadOnly,
    /// Only modify toolkit manifests, don't download packages
    SplitOnly,
}

impl VendorMode {
    fn write_manifest(&self, path: &Path, content: &str) -> Result<()> {
        if !matches!(self, Self::DownloadOnly) {
            fs::write(path, content)?;
        }
        Ok(())
    }

    fn download(&self, url: &str, path: &Path) -> Result<()> {
        download_unless_exists(url, path)?;
        Ok(())
    }
}

pub(super) fn vendor(
    mode: VendorMode,
    name: Option<String>,
    target: Option<String>,
    all_targets: bool,
) -> Result<()> {
    let mut toolkits = Toolkits::load()?;
    gen_manifest_and_download_packages(
        mode,
        &mut toolkits,
        name.as_deref(),
        target.as_deref(),
        all_targets,
    )
}

fn toolkit_needs_downloading(toolkit_name: &str, name_to_dl: Option<&str>) -> bool {
    match name_to_dl {
        Some(s) => s == toolkit_name,
        None => true,
    }
}

fn target_needs_downloading(target: &str, target_to_dl: Option<&str>, all_targets: bool) -> bool {
    if all_targets {
        return true;
    }
    let target_to_dl = target_to_dl.unwrap_or(env!("TARGET"));
    target_to_dl == target
}

/// Reads the `toolkits` value, and:
///
/// - In `SplitOnly` mode, this will write data into `toolkits` value,
///     changing the `url` field of every tool's source to `path`.
/// - In `DownloadOnly` mode, this will just try download the packages to
///     specific location, and will not split `toolkits` into `toolkit-manifest`s.
/// - In `Regular` mode, this does both things above.
fn gen_manifest_and_download_packages(
    mode: VendorMode,
    toolkits: &mut Toolkits,
    name_to_dl: Option<&str>,
    target_to_dl: Option<&str>,
    all_targets: bool,
) -> Result<()> {
    let toolkit_manifests_dir = resources_dir().join("toolkit-manifest");
    let online_manifests_dir = toolkit_manifests_dir.join("online");
    let offline_manifests_dir = toolkit_manifests_dir.join("offline");
    ensure_dir(&online_manifests_dir)?;
    ensure_dir(&offline_manifests_dir)?;

    for (name, toolkit) in &mut toolkits.toolkit {
        let toolkit_needs_downloading = toolkit_needs_downloading(name, name_to_dl);

        let toolkit_root = toolkits.config.abs_package_dir().join(toolkit.full_name());

        // spliting online manifest is easy, because every manifest section was
        // already considered as online manifest, we just need to write its string
        // directly under the right folder.
        let online_manifest = toolkit.manifest_string()?;
        let online_manifest_path = online_manifests_dir.join(format!("{name}.toml"));
        let online_manifest_content = format!("{TOOLSET_MANIFEST_HEADER}{online_manifest}");
        mode.write_manifest(&online_manifest_path, &online_manifest_content)?;

        // offline manifest need some extra steps,
        // first we need to find the `[tools.target]` section,
        // then, we will be changing the tools that have an `url` specified,
        // and change it to a relative `path`
        // (assuming that path is valid, we will use it to download packages).
        let offline_manifest_path = offline_manifests_dir.join(format!("{name}.toml"));
        if let Some(targeted_tools) = toolkit.targeted_tools_mut() {
            for (target, tool) in targeted_tools {
                let tools_dir = toolkit_root.join(target).join(TOOLS_DIRNAME);

                if let Some(tool_info) = tool.as_table_mut() {
                    for (_name, info) in tool_info {
                        let Some(info_table) = info.as_table_mut() else {
                            continue;
                        };
                        if let Some(url) = info_table.get("url").and_then(|v| v.as_str()) {
                            let (_, filename) = url
                                .rsplit_once("/")
                                .ok_or_else(|| anyhow!("missing filename for URL: {url}"))?;
                            let dest = tools_dir.join(filename);
                            let rel_path = format!("{TOOLS_DIRNAME}/{filename}");

                            if toolkit_needs_downloading
                                && target_needs_downloading(target, target_to_dl, all_targets)
                            {
                                ensure_parent_dir(&dest)?;
                                mode.download(url, &dest)?;
                            }

                            info_table.remove("url");
                            info_table.insert("path".into(), toml::Value::String(rel_path));
                        }
                    }
                }
            }
        }
        // Then, insert `[rust.offline-dist-server]` value and `[rust.rustup]` section
        let rust_section = toolkit.rust_section_mut();
        rust_section.insert(
            "offline-dist-server".into(),
            toml::Value::String(TOOLCHAIN_DIRNAME.into()),
        );
        // Make a `[rust.rustup]` map, download rustup-init if necessary
        let mut rustup_sources = IndexMap::new();
        for target in &toolkits.config.targets {
            let triple = target.triple();
            let suffix = if triple.contains("windows") {
                ".exe"
            } else {
                ""
            };
            let value = format!("{TOOLS_DIRNAME}/rustup-init{suffix}");

            if toolkit_needs_downloading
                && target_needs_downloading(triple, target_to_dl, all_targets)
            {
                let rustup_init = format!("rustup-init{suffix}");
                let url = format!(
                    "{}/dist/{triple}/{rustup_init}",
                    toolkits.config.rustup_server,
                );
                let tools_dir = toolkit_root.join(triple).join(TOOLS_DIRNAME);
                ensure_dir(&tools_dir)?;
                let dest = tools_dir.join(rustup_init);
                mode.download(&url, &dest)?;
            }

            rustup_sources.insert(triple.into(), Value::String(value));
        }
        rust_section.insert(
            "rustup".into(),
            toml::Value::Table(Map::from_iter(rustup_sources)),
        );

        // Download rust-toolchain component packages if necessary
        if !matches!(mode, VendorMode::SplitOnly) && toolkit_needs_downloading {
            let toolchain_ver = toolkit.rust_version();
            download_toolchain_components(
                &toolkits.config,
                &toolkit_root,
                toolchain_ver,
                toolkit.date(),
                target_to_dl,
                all_targets,
            )?;
        }

        let offline_manifest = toolkit.manifest_string()?;
        let offline_manifest_content = format!("{TOOLSET_MANIFEST_HEADER}{offline_manifest}");
        mode.write_manifest(&offline_manifest_path, &offline_manifest_content)?;
    }
    Ok(())
}

fn download_toolchain_components(
    config: &GlobalConfig,
    root: &Path,
    version: &str,
    date: &str,
    target_to_dl: Option<&str>,
    all_targets: bool,
) -> Result<()> {
    let targets = &config.targets;
    let components = &config.components;

    for triple in targets.iter().map(|t| t.triple()) {
        if !target_needs_downloading(triple, target_to_dl, all_targets) {
            continue;
        }

        let toolchain_dir = root.join(triple).join(TOOLCHAIN_DIRNAME).join("dist");
        let date_dir = toolchain_dir.join(date);
        ensure_dir(&date_dir)?;

        // download channel manifest first
        let manifest_name = format!("channel-rust-{version}.toml");
        let manifest_hash_name = format!("{manifest_name}.sha256");
        let manifest_src = config.rust_dist_url(&manifest_name);
        let manifest_hash_src = format!("{manifest_src}.sha256");
        let manifest_dest = toolchain_dir.join(manifest_name);
        let manifest_hash_dest = toolchain_dir.join(manifest_hash_name);
        download_unless_exists(&manifest_src, &manifest_dest)?;
        download_unless_exists(&manifest_hash_src, &manifest_hash_dest)?;

        for component in components {
            let comp_name = match component {
                Component::Simple(name) => format!("{name}-{version}-{triple}.tar.xz"),
                Component::Detailed {
                    name,
                    target,
                    wildcard_target,
                    excluded_targets,
                } => {
                    if excluded_targets.contains(triple) {
                        continue;
                    }

                    if *wildcard_target {
                        format!("{name}-{version}.tar.xz")
                    } else if let Some(tg) = target {
                        if !target_needs_downloading(tg, target_to_dl, all_targets) {
                            continue;
                        }
                        format!("{name}-{version}-{tg}.tar.xz")
                    } else {
                        format!("{name}-{version}-{triple}.tar.xz")
                    }
                }
            };
            // let comp_hash_name = format!("{comp_name}.sha256");

            let pkg_src = config.rust_dist_url(&format!("{date}/{comp_name}"));
            // let sha_src = config.rust_dist_url(&format!("{date}/{comp_name}.sha256"));
            let pkg_dest = date_dir.join(&comp_name);
            // let sha_dest = date_dir.join(&comp_hash_name);
            download_unless_exists(&pkg_src, &pkg_dest)?;
            // download_unless_exists(&sha_src, &sha_dest)?;
        }
    }

    Ok(())
}

fn download_unless_exists(src: &str, dest: &Path) -> Result<()> {
    if !dest.is_file() {
        download(src, dest)?;
    }
    Ok(())
}
