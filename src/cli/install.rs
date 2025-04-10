//! Separated module to handle installation related behaviors in command line.

use std::collections::HashSet;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::cli::common::{self, Confirm};
use crate::cli::GlobalOpts;
use crate::components::Component;
use crate::core::install::InstallConfiguration;
use crate::core::{
    default_cargo_registry, default_rustup_dist_server, default_rustup_update_root,
    get_toolkit_manifest, try_it, ToolkitManifestExt,
};
use crate::default_install_dir;

use super::common::{
    question_single_choice, ComponentChoices, ComponentDecoration, ComponentListBuilder,
};
use super::{Installer, ManagerSubcommands};

use anyhow::{bail, Result};
use rim_common::utils;

/// Perform installer actions.
///
/// This will setup the environment and install user selected components.
pub(super) fn execute_installer(installer: &Installer) -> Result<()> {
    let Installer {
        prefix,
        registry_url,
        registry_name,
        rustup_dist_server,
        rustup_update_root,
        manifest: manifest_src,
        insecure,
        list_components,
        component,
        ..
    } = installer;

    if matches!(&prefix, Some(p) if utils::is_root_dir(p)) {
        bail!(t!("notify_root_dir"));
    }

    let manifest_url = manifest_src.as_ref().map(|s| s.to_url()).transpose()?;
    let mut manifest = blocking!(get_toolkit_manifest(manifest_url, *insecure))?;

    if *list_components {
        // print a list of available components then return, don't do anything else
        return super::list::list_components(false, Some(&manifest));
    }

    manifest.adjust_paths()?;

    let component_list = manifest.current_target_components(true)?;
    let abs_prefix = if let Some(path) = prefix {
        utils::to_normalized_absolute_path(path, None)?
    } else {
        default_install_dir()
    };
    let mut user_opt =
        CustomInstallOpt::collect_from_user(&abs_prefix, component_list, component.as_deref())?;

    // fill potentially missing package sources
    manifest.fill_missing_package_source(&mut user_opt.components, ask_tool_source)?;

    let (registry_name, registry_value) = registry_url
        .as_deref()
        .map(|u| (registry_name.as_str(), u))
        .unwrap_or(default_cargo_registry());
    let install_dir = user_opt.prefix;

    InstallConfiguration::new(&install_dir, &manifest)?
        .with_cargo_registry(registry_name, registry_value)
        .with_rustup_dist_server(
            rustup_dist_server
                .clone()
                .unwrap_or_else(|| default_rustup_dist_server().clone()),
        )
        .with_rustup_update_root(
            rustup_update_root
                .clone()
                .unwrap_or_else(|| default_rustup_update_root().clone()),
        )
        .insecure(*insecure)
        .install(user_opt.components)?;

    let g_opts = GlobalOpts::get();
    if !g_opts.quiet {
        println!("\n{}\n", t!("install_finish_info"));
    }

    // NB(J-ZhengLi): the logic is flipped here because...
    // Well, the decision was allowing a `VS-Code` window to popup after installation by default.
    // However, it is not ideal when passing `--yes` when the user just want a quick install,
    // and might gets annoying when the user is doing a 'quick install' on WSL. (a VSCode
    // window will pop open on Windows)
    if !g_opts.yes_to_all && common::confirm(t!("question_try_demo"), true)? {
        try_it::try_it(Some(&install_dir))?;
    }

    #[cfg(unix)]
    if let Some(cmd) = crate::core::os::unix::source_command() {
        if !g_opts.quiet {
            println!("\n{}", t!("linux_source_hint", cmd = cmd));
        }
    }

    Ok(())
}

/// Contains customized install options that will be collected from user input.
///
/// Check [`collect_from_user`](CustomInstallOpt::collect_from_user) for more detail.
#[derive(Debug, Default)]
struct CustomInstallOpt {
    prefix: PathBuf,
    components: Vec<Component>,
}

impl CustomInstallOpt {
    /// Asking various questions and collect user input from console interaction,
    /// then return user specified installation options.
    ///
    /// It takes default values, such as `prefix`, `components`, etc.
    /// and a full list of available components allowing user to choose from.
    fn collect_from_user(
        prefix: &Path,
        all_components: Vec<Component>,
        user_selected_comps: Option<&[String]>,
    ) -> Result<Self> {
        if GlobalOpts::get().yes_to_all {
            return Ok(Self {
                prefix: prefix.to_path_buf(),
                components: default_component_choices(&all_components, user_selected_comps)
                    .values()
                    .map(|c| (*c).to_owned())
                    .collect(),
            });
        }

        // This clear the console screen while also move the cursor to top left
        #[cfg(not(windows))]
        const CLEAR_SCREEN_SPELL: &str = "\x1B[2J\x1B[1:1H";
        #[cfg(windows)]
        const CLEAR_SCREEN_SPELL: &str = "";

        let mut stdout = io::stdout();
        writeln!(
            &mut stdout,
            "{CLEAR_SCREEN_SPELL}\n\n{}",
            t!("welcome", product = utils::build_cfg_locale("product"))
        )?;
        writeln!(&mut stdout, "\n\n{}", t!("what_this_is"))?;
        writeln!(&mut stdout, "{}\n", t!("custom_install_help"))?;

        // initialize these with default value, but they could be altered by the user
        let mut install_dir = utils::path_to_str(prefix)?.to_string();

        loop {
            if let Some(dir_input) = read_install_dir_input(&install_dir)? {
                install_dir = dir_input;
            } else {
                continue;
            }

            let choices = read_component_selections(&all_components, user_selected_comps)?;

            show_confirmation(&install_dir, &choices)?;

            match common::confirm_install()? {
                Confirm::Yes => {
                    return Ok(Self {
                        prefix: install_dir.into(),
                        components: choices.values().map(|c| (*c).to_owned()).collect(),
                    });
                }
                Confirm::No => (),
                Confirm::Abort => std::process::exit(0),
            }
        }
    }
}

fn read_install_dir_input(default: &str) -> Result<Option<String>> {
    let dir_input = common::question_str(t!("question_install_dir"), None, default)?;
    // verify path input before proceeding
    if utils::is_root_dir(&dir_input) {
        warn!("{}", t!("notify_root_dir"));
        Ok(None)
    } else {
        Ok(Some(dir_input))
    }
}

/// Create a collection of component choices base of a filtering condition.
/// Also taking component constrains, such as `requires`, `conflicts` into account.
// TODO: handle conflicts
fn component_choices_with_constrains<'a, F>(
    all_components: &'a [Component],
    condition_callback: F,
) -> ComponentChoices<'a>
where
    F: Fn(usize, &Component) -> bool,
{
    // tracking dependency and conflicting component names.
    // dependencies will be added, and conflicted tools will be removed later.
    let mut dependencies = HashSet::new();

    let mut selections = all_components
        .iter()
        .enumerate()
        .filter(|(idx, c)| {
            let selected = condition_callback(*idx, *c);
            if selected {
                dependencies.extend(&c.requires);
            }
            selected
        })
        .collect::<ComponentChoices>();

    // iterate all components again to add dependencies
    for (idx, comp) in all_components.iter().enumerate() {
        if dependencies.contains(&comp.name) && !comp.installed {
            selections.insert(idx, comp);
        }
    }

    selections
}

fn default_component_choices<'a>(
    all_components: &'a [Component],
    user_selected_comps: Option<&[String]>,
) -> ComponentChoices<'a> {
    let selected_comps_set: HashSet<&String> =
        HashSet::from_iter(user_selected_comps.unwrap_or_default());

    component_choices_with_constrains(all_components, |_idx, component: &Component| -> bool {
        let not_optional_and_not_installed =
            !component.installed && (component.required || !component.optional);
        let user_selected = selected_comps_set.contains(&component.name);
        user_selected || not_optional_and_not_installed
    })
}

fn custom_component_choices<'a>(
    all_components: &'a [Component],
    user_selected_comps: Option<&[String]>,
) -> Result<ComponentChoices<'a>> {
    let list_of_comps = ComponentListBuilder::new(all_components)
        .show_desc(true)
        .decorate(ComponentDecoration::Selection)
        .build();
    let default_ids = default_component_choices(all_components, user_selected_comps)
        .keys()
        .map(|idx| (idx + 1).to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let choices = common::question_multi_choices(
        t!("select_components_to_install"),
        &list_of_comps,
        &default_ids,
    )?;
    // convert input vec to set for faster lookup
    // Note: user input index are started from 1.
    let index_set: HashSet<usize> = choices.into_iter().collect();

    // convert the input indexes to `ComponentChoices`,
    // also we need to add missing `required` tools even if the user didn't choose it.
    Ok(component_choices_with_constrains(
        all_components,
        |idx, c| (c.required && !c.installed) || index_set.contains(&(idx + 1)),
    ))
}

/// Read user response of what set of components they want to install.
///
/// Currently, there's only three options:
/// 1. default
/// 2. everything
/// 3. custom
fn read_component_selections<'a>(
    all_components: &'a [Component],
    user_selected_comps: Option<&[String]>,
) -> Result<ComponentChoices<'a>> {
    let profile_choices = &[
        t!("install_default"),
        t!("install_everything"),
        t!("install_custom"),
    ];
    let choice = question_single_choice(t!("question_components_profile"), profile_choices, "1")?;
    let selection = match choice {
        // Default set
        1 => default_component_choices(all_components, user_selected_comps),
        // Full set, but exclude installed components
        2 => all_components
            .iter()
            .enumerate()
            .filter(|(_, c)| !c.installed)
            .collect(),
        // Customized set
        3 => custom_component_choices(all_components, user_selected_comps)?,
        _ => unreachable!("out-of-range input should already be caught"),
    };

    Ok(selection)
}

fn show_confirmation(install_dir: &str, choices: &ComponentChoices<'_>) -> Result<()> {
    let mut stdout = std::io::stdout();

    writeln!(&mut stdout, "\n{}\n", t!("current_install_option"))?;
    writeln!(&mut stdout, "{}:\n\t{install_dir}", t!("install_dir"))?;
    writeln!(&mut stdout, "\n{}:", t!("components_to_install"))?;
    let list_of_comp = ComponentListBuilder::new(choices.values().copied())
        .decorate(ComponentDecoration::Confirmation)
        .build();
    for line in list_of_comp {
        writeln!(&mut stdout, "\t{line}")?;
    }

    // list obsoleted components
    let obsoletes_removal_list = choices
        .iter()
        .filter_map(|(_, comp)| {
            if !comp.installed {
                return None;
            }
            let mut line = String::new();
            for obsolete in &comp.obsoletes {
                line.push_str(&format!(
                    "\t{obsolete} ({})",
                    t!("replaced_by", name = &comp.name)
                ));
            }
            (!line.is_empty()).then_some(line)
        })
        .collect::<Vec<_>>();
    if !obsoletes_removal_list.is_empty() {
        writeln!(&mut stdout, "\n{}:", t!("components_to_remove"))?;
        for line in obsoletes_removal_list {
            writeln!(&mut stdout, "\t{line}")?;
        }
    }

    Ok(())
}

static SHOW_MISSING_PKG_SRC_ONCE: OnceLock<()> = OnceLock::new();

fn ask_tool_source(name: String) -> Result<String> {
    // print additional info for the first tool
    SHOW_MISSING_PKG_SRC_ONCE.get_or_init(|| {
        let mut stdout = std::io::stdout();
        _ = writeln!(&mut stdout, "\n{}\n", t!("package_source_missing_info"));
    });

    common::question_str(t!("question_package_source", tool = name), None, "")
}

pub(super) fn execute_manager(manager: &ManagerSubcommands) -> Result<bool> {
    let ManagerSubcommands::Install { version, .. } = manager else {
        return Ok(false);
    };

    todo!("install dist with version '{version}'");
}
