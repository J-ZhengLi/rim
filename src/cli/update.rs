use anyhow::{anyhow, Result};
use std::collections::HashSet;
use std::path::Path;
use url::Url;

use crate::components::Component;
use crate::core::toolkit::Toolkit;
use crate::core::update::UpdateOpt;
use crate::core::{get_toolkit_manifest, ToolkitManifestExt};
use crate::toolkit::latest_installable_toolkit;
use crate::InstallConfiguration;

use super::common::{
    ComponentChoices, ComponentDecoration, ComponentListBuilder, VersionDiff, VersionDiffMap,
};
use super::{common, ExecStatus, GlobalOpts, ManagerSubcommands};

pub(super) fn execute(cmd: &ManagerSubcommands) -> Result<ExecStatus> {
    let ManagerSubcommands::Update {
        toolkit_only,
        manager_only,
        insecure,
        component,
        rustup_dist_server,
    } = cmd
    else {
        return Ok(ExecStatus::default());
    };

    let update_opt = UpdateOpt::new().insecure(*insecure);
    if !manager_only {
        update_opt.update_toolkit(|path| {
            blocking!(update_toolkit_(
                path,
                *insecure,
                component.as_deref(),
                rustup_dist_server
            ))
        })?;
    }
    if !toolkit_only {
        blocking!(update_opt.self_update(false))?;
    }

    Ok(ExecStatus::new_executed())
}

async fn update_toolkit_(
    install_dir: &Path,
    insecure: bool,
    user_selected_comps: Option<&[String]>,
    rustup_dist_server: &Option<Url>,
) -> Result<()> {
    let Some(installed) = Toolkit::installed(false).await? else {
        info!("{}", t!("no_toolkit_installed"));
        return Ok(());
    };
    let installed = &*installed.lock().await;

    // get possible update
    let Some(latest_toolkit) = latest_installable_toolkit(installed, insecure).await? else {
        return Ok(());
    };
    log::debug!(
        "detected latest toolkit: {}-{}",
        &latest_toolkit.name,
        &latest_toolkit.version
    );

    // load the latest manifest
    let manifest_url = latest_toolkit
        .manifest_url
        .as_deref()
        .and_then(|s| Url::parse(s).ok())
        .ok_or_else(|| {
            anyhow!(
                "invalid dist manifest downloaded from server: \
            must contains a valid `manifest_url`"
            )
        })?;
    let manifest = get_toolkit_manifest(Some(manifest_url), insecure).await?;
    let new_components = manifest.current_target_components(false)?;

    // notify user that we will install the latest update to replace their current installation
    info!(
        "{}",
        t!(
            "pre_update_note",
            target_version = latest_toolkit.version,
            current_version = installed.version
        )
    );

    let updater = ComponentsUpdater::new(&installed.components, &new_components);
    // let user choose if they want to update installed component only, or want to select more components to install
    if let UpdateOption::Yes(components) = updater.to_update_option(user_selected_comps)? {
        // install update for selected components
        let config = InstallConfiguration::new(install_dir, &manifest)?
            .with_rustup_dist_server(rustup_dist_server.clone());
        config.update(components.into_values().cloned().collect())
    } else {
        Ok(())
    }
}

enum UpdateOption<'c> {
    Yes(ComponentChoices<'c>),
    NoUpdate,
}

struct ComponentsUpdater<'c> {
    target: &'c [Component],
    version_diff: VersionDiffMap<'c>,
}

impl<'c> ComponentsUpdater<'c> {
    fn new(installed: &'c [Component], target: &'c [Component]) -> Self {
        let version_diff = target
            .iter()
            .map(|c| {
                let mut is_installed = false;
                let installed_version = installed
                    .iter()
                    .find_map(|ic| {
                        (ic.name == c.name).then(|| {
                            is_installed = true;
                            ic.version.as_deref()
                        })
                    })
                    .flatten();
                let is_newly_supported = !is_installed && c.version.is_some();
                (
                    c.name.as_str(),
                    VersionDiff {
                        from: installed_version,
                        to: c.version.as_deref(),
                        is_newly_supported,
                    },
                )
            })
            .collect();
        Self {
            target,
            version_diff,
        }
    }

    // We are only pre-selecting the components for update if the component exists in both lists
    // and having different version.
    // Note that we don't check if the new version is actually "newer" than the installed version,
    // it is intended to prevent a scenario where a component needs to be rollback in a new toolkit.
    fn component_names_with_diff_version(&self) -> HashSet<&'c str> {
        self.version_diff
            .iter()
            .filter_map(|(name, diff)| {
                // return only the components that are previously installed
                if diff.is_newly_supported {
                    None
                } else {
                    (diff.from != diff.to).then_some(*name)
                }
            })
            .collect()
    }

    fn to_update_option(&self, user_selected_comps: Option<&[String]>) -> Result<UpdateOption<'c>> {
        let default = self.default_component_choices(user_selected_comps);
        self.handle_update_interaction_(default)
    }

    /// Default component set contains components that:
    /// - User provide a list of components via commandline, such as `--components comp_a,comp_b`.
    /// - Was previously installed and have new version available.
    fn default_component_choices(
        &self,
        user_selected_comps: Option<&[String]>,
    ) -> ComponentChoices<'c> {
        let mut base_set = self.component_names_with_diff_version();
        let mut user_set: HashSet<&str> = HashSet::from_iter(
            user_selected_comps
                .unwrap_or_default()
                .iter()
                .map(|s| s.as_str()),
        );
        let is_append = user_set.remove("..");
        if is_append {
            base_set.extend(user_set);
        } else {
            base_set = user_set;
        }

        self.target
            .iter()
            .enumerate()
            .filter(|(_, c)| base_set.contains(c.name.as_str()))
            .collect()
    }

    fn custom_component_choices(&self, orig: ComponentChoices<'c>) -> Result<ComponentChoices<'c>> {
        let choices = ComponentListBuilder::new(self.target)
            .decorate(ComponentDecoration::VersionDiff(&self.version_diff))
            .show_desc(true)
            .build();
        let default_choices = orig
            .keys()
            .map(|idx| (idx + 1).to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let input = common::question_multi_choices(
            t!("select_components_to_update"),
            &choices,
            default_choices,
        )?;

        // convert input vec to set for faster lookup
        // Note: user input index are started from 1.
        let index_set: HashSet<usize> = input.into_iter().collect();

        // convert the input indexes to `ComponentChoices`
        Ok(self
            .target
            .iter()
            .enumerate()
            .filter(|(idx, _)| index_set.contains(&(idx + 1)))
            .collect())
    }

    // recursively ask for user input
    fn handle_update_interaction_(&self, list: ComponentChoices<'c>) -> Result<UpdateOption<'c>> {
        if GlobalOpts::get().yes_to_all {
            return Ok(UpdateOption::Yes(list));
        }

        let choices = vec![t!("continue"), t!("customize"), t!("cancel")];
        let comp_list = ComponentListBuilder::new(list.values().copied())
            .decorate(ComponentDecoration::VersionDiff(&self.version_diff))
            .build()
            .join("\n");
        let choice = common::question_single_choice(
            t!("pre_update_confirmation", list = comp_list),
            &choices,
            1,
        )?;
        match choice {
            1 => Ok(UpdateOption::Yes(list)),
            2 => {
                let custom_choices = self.custom_component_choices(list)?;
                self.handle_update_interaction_(custom_choices)
            }
            3 => Ok(UpdateOption::NoUpdate),
            _ => {
                unreachable!("input function should already catches out of range input '{choice}'")
            }
        }
    }
}
