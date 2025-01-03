//! Contains all the definition of command line arguments.

mod common;
mod component;
mod install;
mod list;
mod tryit;
mod uninstall;
mod update;

use anyhow::{anyhow, bail, Result};
use clap::{Parser, Subcommand, ValueHint};
use common::handle_user_choice;
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};
use url::Url;

use crate::{
    core::{GlobalOpts, Language},
    utils,
};
pub use common::pause;

/// Install rustup, rust toolchain, and various tools.
// NOTE: If you changed anything in this struct, or any other child types that related to
// this struct, make sure the README doc is updated as well,
#[derive(Parser, Default, Debug)]
#[command(version, about)]
pub struct Installer {
    /// Enable verbose output
    #[arg(short, long, conflicts_with = "quiet")]
    pub verbose: bool,
    /// Suppress non-critical messages
    #[arg(short, long, conflicts_with = "verbose")]
    pub quiet: bool,
    /// Disable interaction and answer 'yes' to all prompts
    #[arg(short, long = "yes")]
    yes_to_all: bool,
    #[cfg(feature = "gui")]
    /// Don't show GUI when running the program.
    #[arg(hide = true, long)]
    pub no_gui: bool,
    /// Don't modify user's `PATH` environment variable.
    ///
    /// Note that some other variables (such as CARGO_HOME, RUSTUP_DIST_SERVER, etc.)
    /// will still be written to ensure the Rust toolchain can be used correctly.
    #[arg(long)]
    no_modify_path: bool,
    /// Don't make any environment modifications on user's machine,
    /// including Windows registry entries and `PATH` variable.
    ///
    /// Note that the installation might not work as intended if some
    /// of the variables are missing (such as CARGO_HOME, RUSTUP_DIST_SERVER, etc.).
    /// Do NOT use this if you don't know what you're doing.
    #[arg(long, conflicts_with = "no_modify_path")]
    no_modify_env: bool,
    /// Allow insecure connections when download packages from server.
    #[arg(short = 'k', long)]
    insecure: bool,

    /// Specify another language to display
    #[arg(short, long, value_name = "LANG", value_parser = Language::possible_values())]
    pub lang: Option<String>,
    /// Set another path to install Rust.
    #[arg(long, value_name = "PATH", value_hint = ValueHint::DirPath)]
    prefix: Option<PathBuf>,
    /// Specify another cargo registry url to replace `crates.io`, could be `sparse+URL`.
    #[arg(hide = true, long)]
    registry_url: Option<String>,
    /// Specify another cargo registry name to replace `crates.io`.
    #[arg(hide = true, long, default_value = "mirror")]
    registry_name: String,
    /// Specify another server to download Rust toolchain.
    #[arg(hide = true, long, value_name = "URL", value_hint = ValueHint::Url)]
    rustup_dist_server: Option<Url>,
    /// Specify another server to download rustup.
    #[arg(hide = true, long, value_name = "URL", value_hint = ValueHint::Url)]
    rustup_update_root: Option<Url>,
    /// Specify a path or url of manifest file that contains package source and various configurations.
    #[arg(long, value_name = "PATH or URL")]
    manifest: Option<PathOrUrl>,
    /// Display a list of components that can be installed on current machine.
    #[arg(long, conflicts_with = "component")]
    list_components: bool,
    /// Include a list of components (separated by comma) to install.
    /// Note that required components will be installed whether included or not.
    ///
    /// For the complete list, use `--list-components` option.
    #[arg(short, long, value_delimiter = ',')]
    component: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub(crate) enum PathOrUrl {
    Path(PathBuf),
    Url(Url),
}

impl FromStr for PathOrUrl {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        if let Ok(abs_path) = utils::to_nomalized_abspath(s, None) {
            if !abs_path.exists() {
                bail!("the specified path '{s}' does not exist");
            }
            Ok(PathOrUrl::Path(abs_path))
        } else {
            Ok(PathOrUrl::Url(Url::parse(s)?))
        }
    }
}

impl PathOrUrl {
    /// Extract [`Url`] value or convert [`PathBuf`] to [`Url`] with file scheme.
    ///
    /// # Error
    /// This will fail when trying to convert a relative path.
    fn to_url(&self) -> Result<Url> {
        match self {
            Self::Url(url) => Ok(url.clone()),
            Self::Path(path) => {
                Url::from_file_path(path).map_err(|_| anyhow!("invalid path '{}'", path.display()))
            }
        }
    }
}

/// Manage Rust installation, mostly used for uninstalling.
// NOTE: If you changed anything in this struct, or any other child types that related to
// this struct, make sure the README doc is updated as well,
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Manager {
    /// Enable verbose output
    #[arg(short, long, conflicts_with = "quiet")]
    pub verbose: bool,
    /// Suppress non-critical messages
    #[arg(short, long, conflicts_with = "verbose")]
    pub quiet: bool,
    /// Disable interaction and answer 'yes' to all prompts
    #[arg(short, long = "yes")]
    yes_to_all: bool,
    #[cfg(feature = "gui")]
    /// Don't show GUI when running the program.
    #[arg(hide = true, long)]
    pub no_gui: bool,
    /// Don't modify user's `PATH` environment variable.
    #[arg(long)]
    no_modify_path: bool,
    /// Don't make any environment modifications on user's machine.
    ///
    /// This includes environment veriables including `PATH`, `CARGO_HOME`, `RUSTUP_HOME` etc,
    /// keeping them intact even after uninstallation.
    /// This Does not includes Windows `Uninstall` entry of course, which will get removed after
    /// uninstallation.
    #[arg(long, conflicts_with = "no_modify_path")]
    no_modify_env: bool,

    /// Specify another language to display
    #[arg(short, long, value_name = "LANG", value_parser = Language::possible_values())]
    pub lang: Option<String>,
    #[command(subcommand)]
    command: Option<ManagerSubcommands>,
}

impl Installer {
    pub fn install_dir(&self) -> Option<&Path> {
        self.prefix.as_deref()
    }

    pub fn manifest_url(&self) -> Result<Option<Url>> {
        self.manifest.as_ref().map(|m| m.to_url()).transpose()
    }

    pub fn execute(&self) -> Result<()> {
        setup(
            self.verbose,
            self.quiet,
            self.yes_to_all,
            self.no_modify_env,
            self.no_modify_path,
            self.lang.as_deref(),
        )?;

        install::execute_installer(self)
    }
}

impl Manager {
    pub fn execute(&self) -> Result<()> {
        setup(
            self.verbose,
            self.quiet,
            self.yes_to_all,
            self.no_modify_env,
            self.no_modify_path,
            self.lang.as_deref(),
        )?;

        let Some(subcmd) = &self.command else {
            return ManagerSubcommands::from_interaction()?.execute();
        };
        subcmd.execute()
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Subcommand, Debug)]
enum ManagerSubcommands {
    /// Install a specific dist version
    #[command(hide = true)]
    Install {
        /// Allow insecure connections when download packages from server.
        #[arg(short = 'k', long)]
        insecure: bool,
        #[arg(value_name = "VERSION")]
        version: String,
    },
    /// Update toolkit and/or this installation manager
    ///
    /// By default, this will update both the toolkit and manager, if you just want to update
    /// on of them, pass `--<toolkit|manager>-only` option to it.
    Update {
        /// Allow insecure connections when download packages from server.
        #[arg(short = 'k', long)]
        insecure: bool,
        /// Update toolkit only
        #[arg(long, alias = "toolkit", conflicts_with = "manager_only")]
        toolkit_only: bool,
        /// Update manager only
        #[arg(long, alias = "manager")]
        manager_only: bool,
    },
    /// Display a list of toolkits or components
    List {
        /// Show installed only
        #[arg(long)]
        installed: bool,
        #[command(subcommand)]
        command: Option<list::ListCommand>,
    },
    #[command(hide = true)]
    /// Install or uninstall components
    Component {
        #[command(subcommand)]
        command: component::ComponentCommand,
    },
    /// Uninstall individual components or everything.
    Uninstall {
        /// Keep this manager tool, only uninstall toolkit
        #[arg(long, alias = "keep-manager")]
        keep_self: bool,
    },
    /// A subcommand to create a new Rust project template and let you start coding with it.
    TryIt {
        /// Specify another directory to create project template, defaulting to current directory.
        #[arg(long, short, value_name = "PATH", value_hint = ValueHint::DirPath)]
        path: Option<PathBuf>,
    },
}

macro_rules! return_if_executed {
    ($($fn:expr),+) => {
        $(
            if $fn {
                return Ok(());
            }
        )*
    };
}

impl ManagerSubcommands {
    pub(crate) fn execute(&self) -> Result<()> {
        return_if_executed! {
            install::execute_manager(self)?,
            update::execute(self)?,
            list::execute(self)?,
            component::execute(self)?,
            uninstall::execute(self)?,
            tryit::execute(self)?
        }
        Ok(())
    }

    fn from_interaction() -> Result<Self> {
        loop {
            let Some(mut manager_opt) = Self::question_manager_option_()? else {
                // user choose to cancel, exit the program
                std::process::exit(0);
            };

            match manager_opt {
                Self::Update { insecure, .. } => {
                    if !manager_opt.question_update_option_(insecure)? {
                        continue;
                    }
                }
                Self::Uninstall { .. } => {
                    if !manager_opt.question_uninstall_option_()? {
                        continue;
                    }
                }
                Self::List { command, .. } => {
                    if command.is_none() {
                        continue;
                    }
                }
                _ => unimplemented!(
                    "manager interaction currently only have `update` and `install` \
                    options available"
                ),
            }

            return Ok(manager_opt);
        }
    }

    fn question_manager_option_() -> Result<Option<Self>> {
        // NOTE: If more option added, make sure to add the corresponding match pattern
        // to `from_interaction` function., otherwise it may cause `unimplemented` error.
        let maybe_cmd = handle_user_choice!(
            t!("choose_an_option"), 4,
            {
                1 t!("update") => {
                    let insecure = handle_user_choice!(
                        t!("choose_an_option"), 1,
                        {
                            1 t!("default") => { false },
                            2 t!("skip_ssl_check") => { true }
                        }
                    );
                    Some(Self::Update { insecure, toolkit_only: false, manager_only: false })
                },
                2 t!("uninstall") => { Some(Self::Uninstall { keep_self: false }) },
                3 t!("list_option") => {
                    let installed = handle_user_choice!(
                        t!("choose_an_option"), 1,
                        {
                            1 t!("all") => { false },
                            2 t!("installed") => { true }
                        }
                    );
                    Some(Self::List { installed, command: list::ask_list_command()? })
                },
                4 t!("cancel") => { None }
            }
        );

        Ok(maybe_cmd)
    }

    /// Ask user about the update options, return a `bool` indicates whether the
    /// user wishs to continue.
    fn question_update_option_(&mut self, insecure: bool) -> Result<bool> {
        *self = handle_user_choice!(
            t!("choose_an_option"), 1,
            {
                1 t!("update_all") => {
                    Self::Update { insecure, toolkit_only: false, manager_only: false }
                },
                2 t!("update_self_only") => {
                    Self::Update { insecure, toolkit_only: false, manager_only: true }
                },
                3 t!("update_toolkit_only") => {
                    Self::Update { insecure, toolkit_only: true, manager_only: false }
                },
                4 t!("back") => { return Ok(false) }
            }
        );

        Ok(true)
    }

    /// Ask user about uninstallation options, return a `bool` indicates whether the
    /// user wishs to continue.
    fn question_uninstall_option_(&mut self) -> Result<bool> {
        *self = handle_user_choice!(
            t!("choose_an_option"), 1,
            {
                1 t!("uninstall_all") => { Self::Uninstall { keep_self: false } },
                2 t!("uninstall_toolkit_only") => { Self::Uninstall { keep_self: true } },
                3 t!("back") => { return Ok(false) }
            }
        );

        Ok(true)
    }
}

pub fn parse_installer_cli() -> Installer {
    Installer::parse()
}

pub fn parse_manager_cli() -> Manager {
    Manager::parse()
}

fn setup(
    verbose: bool,
    quiet: bool,
    yes: bool,
    no_modify_env: bool,
    no_modify_path: bool,
    lang: Option<&str>,
) -> Result<()> {
    // Setup locale
    if let Some(lang_str) = lang {
        let parsed: Language = lang_str.parse()?;
        utils::set_locale(parsed.locale_str());
    } else {
        utils::use_current_locale();
    }
    // Setup logger
    utils::Logger::new().verbose(verbose).quiet(quiet).setup()?;
    // Setup global options
    GlobalOpts::set(verbose, quiet, yes, no_modify_env, no_modify_path);

    Ok(())
}
