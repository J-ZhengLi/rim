//! Contains all the definition of command line arguments.

mod check;
mod common;
mod component;
mod install;
mod list;
mod tryit;
mod uninstall;
mod update;

use crate::core::GlobalOpts;
use anyhow::{anyhow, bail, Result};
use clap::error::ErrorKind;
use clap::{Parser, Subcommand, ValueHint};
use common::handle_user_choice;
use rim_common::types::Language;
use rim_common::utils;
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};
use url::Url;

// Re-exports
pub use common::pause;
pub use component::ComponentCommand;

/// Receive a list of function calls, that waits to be executed,
/// if any of them returning [`ExecStatus`] which the `executed` flag is `true`,
/// this will trigger an return action.
///
/// Therefore make sure the return type of each function call and the upper function
/// of this macro call are all `Result<ExecStatus>`.
///
/// # Example
/// ```ignore
/// fn foo(x: i32) -> Result<ExecStatus> {
///     Ok(ExecStatus::default())
/// }
/// fn bar(x: i32) -> Result<ExecStatus> {
///     if x = 1 {
///         Ok(ExecStatus::new_executed())
///     } else {
///         Ok(ExecStatus::default())
///     }
/// }
/// fn baz(x: i32) -> Result<ExecStatus> {
///     Ok(ExecStatus::new_executed())
/// }
///
/// let x = 1;
/// // This will first run `foo(x)`, which returns `Ok(ExecStatus::default())`,
/// // thus the `executed` flag is `false`, so the code continues to run.
/// // Then the `bar(x)` gets to run, and because `x` is `1`, causing `ExecStatus::new_executed()`
/// // being returned, therefore the whole block returns and `baz(x)` was never run.
/// return_if_executed {
///     foo(x),
///     bar(x),
///     baz(x),
/// }
/// ```
macro_rules! return_if_executed {
    ($($fn:expr),+) => {
        $(
            let _res_ = $fn;
            if _res_.executed {
                return Ok(_res_);
            }
        )*
    };
}

/// Provides an `execute` function to run `clap` compatible commands.
pub trait ExecutableCommand {
    /// Executes a command and return [`ExecStatus`].
    fn execute(&self) -> Result<ExecStatus>;

    /// Return `true` if this command should be running in silent mode.
    /// (without any interface)
    fn silent_mode(&self) -> bool {
        false
    }

    /// Return `true` if the command has specify not to start graphical
    /// interface while executing.
    fn no_gui(&self) -> bool {
        false
    }
}

impl<T: ExecutableCommand> ExecutableCommand for Box<T> {
    fn execute(&self) -> Result<ExecStatus> {
        self.as_ref().execute()
    }
    fn silent_mode(&self) -> bool {
        self.as_ref().silent_mode()
    }
    fn no_gui(&self) -> bool {
        self.as_ref().no_gui()
    }
}

/// Indicates the status after running a [`ExecutableCommand`].
///
/// # Note
/// The status mainly indicates two things:
/// 1. `executed`, this flag is used to determine whether a command was executed
///    or not (obviously). Because this program supports multiple commands and subcommands,
///    we can have a list of these command's execute function to wait for run,
///    and return only if one of them was executed. Check [`return_if_executed`] for example.
/// 2. `no_pause`, on Windows, when user double clicks our main executable,
///    without manually pausing, the window will open then immediately closes after
///    executing, making error reporting extremely hard.
#[derive(Debug, Clone, Default)]
pub struct ExecStatus {
    /// Whether the command was executed or not.
    pub executed: bool,
    /// A flag to skip pausing the console window (on Windows) after executing.
    pub no_pause: bool,
}

impl ExecStatus {
    /// Create a new `ExecStatus` and mark it as `executed`
    pub fn new_executed() -> Self {
        Self {
            executed: true,
            ..Default::default()
        }
    }

    setter!(executed(self.executed, bool));
    setter!(no_pause(self.no_pause, bool));
}

fn possible_lang() -> Vec<&'static str> {
    Language::possible_values()
        .iter()
        .map(Language::as_str)
        .collect()
}

/// Install rustup, rust toolchain, and various tools.
// NOTE: If you changed anything in this struct, or any other child types that related to
// this struct, make sure the README doc is updated as well,
#[derive(Parser, Default, Debug)]
#[command(
    version = common::version(),
    about
)]
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
    #[arg(long)]
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
    #[arg(short, long, value_name = "LANG", value_parser = possible_lang())]
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
    pub rustup_dist_server: Option<Url>,
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
        if let Ok(abs_path) = utils::to_normalized_absolute_path(s, None) {
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
#[derive(Parser, Debug, Clone)]
#[command(
    version = common::version(),
    about
)]
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
    #[arg(long)]
    pub no_gui: bool,
    #[cfg(feature = "gui")]
    /// Run manager without showing the main window
    #[arg(short, long)]
    pub silent: bool,
    /// Don't modify user's `PATH` environment variable.
    #[arg(long)]
    no_modify_path: bool,
    /// Don't make any environment modifications on user's machine.
    ///
    /// This includes environment variables including `PATH`, `CARGO_HOME`, `RUSTUP_HOME` etc,
    /// keeping them intact even after uninstallation.
    /// This Does not includes Windows `Uninstall` entry of course, which will get removed after
    /// uninstallation.
    #[arg(long, conflicts_with = "no_modify_path")]
    no_modify_env: bool,

    /// Specify another language to display
    #[arg(short, long, value_name = "LANG", value_parser = possible_lang())]
    pub lang: Option<String>,
    #[command(subcommand)]
    pub command: Option<ManagerSubcommands>,
}

impl Installer {
    pub fn install_dir(&self) -> Option<&Path> {
        self.prefix.as_deref()
    }
}

impl ExecutableCommand for Installer {
    fn execute(&self) -> Result<ExecStatus> {
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

    #[cfg(feature = "gui")]
    fn no_gui(&self) -> bool {
        self.no_gui
    }
}

impl ExecutableCommand for Manager {
    fn execute(&self) -> Result<ExecStatus> {
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

    #[cfg(feature = "gui")]
    fn silent_mode(&self) -> bool {
        self.silent
    }

    #[cfg(feature = "gui")]
    fn no_gui(&self) -> bool {
        if self.no_gui {
            return true;
        }

        // (manager only) If any of these subcommand was invoked, do not start GUI
        matches!(
            self.command,
            Some(ManagerSubcommands::Check { .. } | ManagerSubcommands::TryIt { .. })
        )
    }
}

impl TryFrom<Vec<String>> for Manager {
    type Error = anyhow::Error;

    fn try_from(value: Vec<String>) -> Result<Self> {
        Ok(Self::try_parse_from(value)?)
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Subcommand, Debug, Clone)]
pub enum ManagerSubcommands {
    /// Install a specific dist version
    #[command(hide = true)]
    Install {
        /// Allow insecure connections when download packages from server.
        #[arg(short = 'k', long)]
        insecure: bool,
        #[arg(value_name = "VERSION")]
        version: String,
        /// Specify another server to download Rust toolchain components.
        #[arg(long, value_name = "URL", value_hint = ValueHint::Url)]
        rustup_dist_server: Option<Url>,
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
        /// Include a list of components (separated by comma) to update,
        /// effective only when updating toolkit.
        ///
        /// By default, the value of this option will override the list of components
        /// to be updated, meaning if you use `--component a,b`, only component a and b will
        /// be updated.
        /// If you want to keep the default selection, but adding some extra components to update,
        /// you need to include a `..` in the value, such as `--component a,b,..`, then not only
        /// a and b, but also other components that were selected by default will get updated.
        #[arg(short, long, value_delimiter = ',')]
        component: Option<Vec<String>>,
        /// Specify another server to download Rust toolchain components.
        #[arg(long, value_name = "URL", value_hint = ValueHint::Url)]
        rustup_dist_server: Option<Url>,
    },
    /// Display a list of toolkits or components
    List {
        /// Show installed only
        #[arg(long)]
        installed: bool,
        #[command(subcommand)]
        command: Option<list::ListCommand>,
    },
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
    /// Check source code in the current directory using installed rule-set for errors
    Check {
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            value_name = "ARGS"
        )]
        /// Additional args to run `cargo clippy`, see all options with `cargo clippy --help`.
        extra_args: Vec<String>,
    },
}

impl ExecutableCommand for ManagerSubcommands {
    fn execute(&self) -> Result<ExecStatus> {
        return_if_executed! {
            install::execute_manager(self)?,
            update::execute(self)?,
            list::execute(self)?,
            component::execute(self)?,
            uninstall::execute(self)?,
            tryit::execute(self)?,
            check::execute(self)?
        }
        Ok(ExecStatus::default())
    }
}

impl ManagerSubcommands {
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
                Self::Component { .. } => {
                    if !manager_opt.question_component_option_()? {
                        continue;
                    }
                }
                _ => unimplemented!("manager interaction does not support this option yet"),
            }

            return Ok(manager_opt);
        }
    }

    fn question_manager_option_() -> Result<Option<Self>> {
        // NOTE: If more option added, make sure to add the corresponding match pattern
        // to `from_interaction` function., otherwise it may cause `unimplemented` error.
        // NOTE: Don't get confused by the options in `Self` variant being returned, they
        // are dummy options, and will get replaced in further interaction.
        let maybe_cmd = handle_user_choice!(
            t!("choose_an_option"), 5,
            {
                1 t!("modify_option") => { Some(Self::Component { command: ComponentCommand::Uninstall { components: vec![] } }) },
                2 t!("update") => {
                    let insecure = handle_user_choice!(
                        t!("choose_an_option"), 1,
                        {
                            1 t!("default") => { false },
                            2 t!("skip_ssl_check") => { true }
                        }
                    );
                    Some(Self::Update { insecure, toolkit_only: false, manager_only: false, component: None, rustup_dist_server: None })
                },
                3 t!("uninstall") => { Some(Self::Uninstall { keep_self: false }) },
                4 t!("list_option") => {
                    let installed = handle_user_choice!(
                        t!("choose_an_option"), 1,
                        {
                            1 t!("all") => { false },
                            2 t!("installed") => { true }
                        }
                    );
                    Some(Self::List { installed, command: list::ask_list_command()? })
                },
                5 t!("cancel") => { None }
            }
        );

        Ok(maybe_cmd)
    }

    /// Ask user about the update options, return a `bool` indicates whether the
    /// user wish to continue.
    fn question_update_option_(&mut self, insecure: bool) -> Result<bool> {
        // component choices are asked after executing update command,
        // so it's ok to leave it as None for now.
        let component = None;
        *self = handle_user_choice!(
            t!("choose_an_option"), 1,
            {
                1 t!("update_all") => {
                    Self::Update { insecure, toolkit_only: false, manager_only: false, component, rustup_dist_server: None }
                },
                2 t!("update_self_only") => {
                    Self::Update { insecure, toolkit_only: false, manager_only: true, component, rustup_dist_server: None }
                },
                3 t!("update_toolkit_only") => {
                    Self::Update { insecure, toolkit_only: true, manager_only: false, component, rustup_dist_server: None}
                },
                4 t!("back") => { return Ok(false) }
            }
        );

        Ok(true)
    }

    /// Ask user about uninstallation options, return a `bool` indicates whether the
    /// user wish to continue.
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

    fn question_component_option_(&mut self) -> Result<bool> {
        *self = handle_user_choice!(
            t!("choose_an_option"), 1,
            {
                1 t!("add") => {
                    let insecure = handle_user_choice!(
                        t!("choose_an_option"), 1,
                        {
                            1 t!("default") => { false },
                            2 t!("skip_ssl_check") => { true }
                        }
                    );
                    let components = component::collect_components_to_add()?;
                    if components.is_empty() {
                        info!("{}", t!("no_component_selected"));
                        return Ok(false);
                    }
                    Self::Component { command: ComponentCommand::Install { insecure, components, rustup_dist_server: None } }
                },
                2 t!("remove") => {
                    let components = component::collect_components_to_remove()?;
                    Self::Component { command: ComponentCommand::Uninstall { components } }
                },
                3 t!("back") => { return Ok(false) }
            }
        );
        Ok(true)
    }
}

/// Parsing commandline args for `Installer` mode.
pub fn parse_installer_cli() -> Result<Box<Installer>> {
    Installer::try_parse()
        .map(Box::new)
        .map_err(manually_show_help_or_version)
}

/// Parsing commandline args for `Manager` mode.
pub fn parse_manager_cli() -> Result<Box<Manager>> {
    Manager::try_parse()
        .map(Box::new)
        .map_err(manually_show_help_or_version)
}

fn manually_show_help_or_version(error: clap::Error) -> anyhow::Error {
    match error.kind() {
        ErrorKind::DisplayHelp
        | ErrorKind::DisplayVersion
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => error.exit(),
        _ => error.into(),
    }
}

fn setup(
    verbose: bool,
    quiet: bool,
    yes: bool,
    no_modify_env: bool,
    no_modify_path: bool,
    lang: Option<&str>,
) -> Result<()> {
    // Setup logger
    utils::Logger::new().verbose(verbose).quiet(quiet).setup()?;

    // Setup locale
    if let Some(lang_str) = lang {
        let parsed: Language = lang_str.parse()?;
        utils::set_locale(parsed.locale_str());
    } else {
        utils::use_current_locale();
    }
    // Setup global options
    GlobalOpts::set(verbose, quiet, yes, no_modify_env, no_modify_path);

    Ok(())
}
