use crate::{
    common::{download, resources_dir},
    toolkits_parser::{Component, Configuration, Toolkits},
};
use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use rim_common::{
    types::{ToolInfo, ToolSource},
    utils::{ensure_dir, ensure_parent_dir},
};
use std::{fs, path::Path};

const TOOLS_DIRNAME: &str = "tools";
const TOOLCHAIN_DIRNAME: &str = "toolchain";

pub(super) const VENDOR_HELP: &str = r#"
Split `toolkits.toml` and download packages specified in it for offline packaging

Usage: cargo dev vendor [OPTIONS]

Options:
    -n, --name      Specify another name of toolkit to download packages for
    -a, --all-targets
                    Download packages for all supporting targets
    -c, --clear     Clear the previously downloaded packages
        --for       Specify the target(s) to downloading packages for, defaulting to current running target
        --download-only
                    Do not update toolkit-manifests, just download packages
        --split-only
                    Update toolkit-manifests by splitting the `toolkits.toml` under resources folder, but don't download packages.
                    Note that splitting will generate offline toolset-manifest as well,
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

/// Helper struct to combine all options that passed from CLI.
struct VendorArgs {
    mode: VendorMode,
    name: Option<String>,
    targets: Vec<String>,
    /// Whether packages of all supported targets should be downloaded.
    all_targets: bool,
    clear: bool,
}

impl VendorArgs {
    fn should_download_for_target(&self, target: &str) -> bool {
        !matches!(self.mode, VendorMode::SplitOnly)
            && (self.all_targets || self.targets.iter().any(|s| s == target))
    }

    fn should_clear_previous_downloads(&self, toolkit_name: &str) -> bool {
        self.clear && self.name.as_deref().unwrap_or(env!("EDITION")) == toolkit_name
    }

    fn should_download(&self, toolkit_name: &str, target: &str) -> bool {
        self.should_download_for_target(target)
            && self.name.as_deref().unwrap_or(env!("EDITION")) == toolkit_name
    }

    fn write_manifest_if_needed(&self, path: &Path, content: &str) -> Result<()> {
        if !matches!(self.mode, VendorMode::DownloadOnly) {
            fs::write(path, content)?;
        }
        Ok(())
    }
}

pub(super) fn vendor(
    mode: VendorMode,
    name: Option<String>,
    targets: Vec<String>,
    all_targets: bool,
    clear: bool,
) -> Result<()> {
    let args = VendorArgs {
        mode,
        name,
        targets,
        all_targets,
        clear,
    };
    let mut toolkits = Toolkits::load()?;
    gen_manifest_and_download_packages(&args, &mut toolkits)
}

/// Reads the `toolkits` value, and:
///
/// - In `SplitOnly` mode, this will write data into `toolkits` value,
///   changing the `url` field of every tool's source to `path`.
/// - In `DownloadOnly` mode, this will just try download the packages to
///   specific location, and will not split `toolkits` into `toolkit-manifest`s.
/// - In `Regular` mode, this does both things above.
fn gen_manifest_and_download_packages(args: &VendorArgs, toolkits: &mut Toolkits) -> Result<()> {
    let toolkit_manifests_dir = resources_dir().join("toolkit-manifest");
    let online_manifests_dir = toolkit_manifests_dir.join("online");
    let offline_manifests_dir = toolkit_manifests_dir.join("offline");
    ensure_dir(&online_manifests_dir)?;
    ensure_dir(&offline_manifests_dir)?;

    for (name, toolkit) in &mut toolkits.toolkit {
        let config = &toolkit.overridden_config(&toolkits.config);
        let toolkit_root = config.abs_package_dir().join(toolkit.full_name());
        if args.should_clear_previous_downloads(name) && toolkit_root.is_dir() {
            println!(
                "removing previously downloaded packages under: {:?}",
                toolkit_root.display()
            );
            fs::remove_dir_all(&toolkit_root)?;
        }

        // splitting online manifest is easy, because every manifest section was
        // already considered as online manifest, we just need to write its string
        // directly under the right folder.
        let online_manifest = toolkit.manifest_string()?;
        let online_manifest_path = online_manifests_dir.join(format!("{name}.toml"));
        let online_manifest_content = format!("{TOOLSET_MANIFEST_HEADER}{online_manifest}");
        args.write_manifest_if_needed(&online_manifest_path, &online_manifest_content)?;

        // offline manifest need some extra steps,
        // first we need to find the `[tools.target]` section,
        // then, we will be changing the tools that have an `url` specified,
        // and change it to a relative `path`
        // (assuming that path is valid, we will use it to download packages).
        let offline_manifest_path = offline_manifests_dir.join(format!("{name}.toml"));
        let targeted_tools = &mut toolkit.manifest.tools.target;
        for (target, tool_info) in targeted_tools {
            let tools_dir = toolkit_root.join(target).join(TOOLS_DIRNAME);

            for (_name, info_table) in tool_info.iter_mut() {
                let ToolInfo::Complex(details) = info_table else {
                    continue;
                };
                if let Some(ToolSource::Url {
                    version: _,
                    url,
                    filename,
                }) = &details.source
                {
                    let filename = if let Some(name) = filename {
                        name
                    } else {
                        url.as_str()
                            .rsplit_once("/")
                            .ok_or_else(|| anyhow!("missing filename for URL: {url}"))?
                            .1
                    };
                    let rel_path = format!("{TOOLS_DIRNAME}/{filename}");

                    if args.should_download(name, target) {
                        let dest = tools_dir.join(filename);
                        ensure_parent_dir(&dest)?;
                        download(url.as_str(), &dest)?;
                    }

                    // convert url package source to path.
                    info_table.url_to_path(rel_path);
                }
            }
        }
        // Then, insert `[rust.offline-dist-server]` value and `[rust.rustup]` section
        let rust_section = &mut toolkit.manifest.rust;
        rust_section.offline_dist_server = Some(TOOLCHAIN_DIRNAME.into());
        // Make a `[rust.rustup]` map, download rustup-init if necessary
        let mut rustup_sources = IndexMap::new();
        for target in &config.targets {
            let triple = target.triple();
            let suffix = if triple.contains("windows") {
                ".exe"
            } else {
                ""
            };
            let value = format!("{TOOLS_DIRNAME}/rustup-init{suffix}");

            if args.should_download(name, triple) {
                let rustup_init = format!("rustup-init{suffix}");
                let url = config.rustup_dist_url(&format!("{triple}/{rustup_init}"));
                let tools_dir = toolkit_root.join(triple).join(TOOLS_DIRNAME);
                ensure_dir(&tools_dir)?;
                let dest = tools_dir.join(rustup_init);
                download(&url, &dest)?;
            }

            rustup_sources.insert(triple.into(), value);
        }
        rust_section.rustup = rustup_sources;

        // Download rust-toolchain component packages if necessary
        for target in &config.targets {
            let triple = target.triple();
            if !args.should_download(name, triple) {
                continue;
            }

            download_toolchain_components(
                config,
                &toolkit_root,
                toolkit.rust_version(),
                toolkit.date(),
                triple,
                args,
            )?;
        }

        let offline_manifest = toolkit.manifest_string()?;
        let offline_manifest_content = format!("{TOOLSET_MANIFEST_HEADER}{offline_manifest}");
        args.write_manifest_if_needed(&offline_manifest_path, &offline_manifest_content)?;
    }
    Ok(())
}

fn download_toolchain_components(
    config: &Configuration,
    root: &Path,
    version: &str,
    date: &str,
    triple: &str,
    args: &VendorArgs,
) -> Result<()> {
    let components = &config.components;

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
    download(&manifest_src, &manifest_dest)?;
    download(&manifest_hash_src, &manifest_hash_dest)?;

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
                    if !args.should_download_for_target(tg) {
                        continue;
                    }
                    format!("{name}-{version}-{tg}.tar.xz")
                } else {
                    format!("{name}-{version}-{triple}.tar.xz")
                }
            }
        };

        let pkg_src = config.rust_dist_url(&format!("{date}/{comp_name}"));
        let pkg_dest = date_dir.join(&comp_name);
        download(&pkg_src, &pkg_dest)?;
    }

    Ok(())
}
