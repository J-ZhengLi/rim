//! Common API for CLI interactions.
//!
//! You might think some of these look like they are from rustup.
//! You are god d*mn right!
//!                         --- Walter White

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    io::{self, BufRead, Write},
};

use anyhow::{Context, Result};
use indexmap::IndexMap;

use crate::components::Component;

/// A "convenient" helper macro to [`question_single_choice`].
///
/// This wraps all things one typically needs, such as storing user's input
/// then handling the action of each individual choice.
///
/// # Example
/// ```ignore
/// let s: &str;
///
/// handle_user_choice!(
///     "choose a word to greet",
///     // 1 is the default value
///     1,
///     s => {
///         1 "world" => { "hello world!" },
///         2 "rust" => { "hello rust!" },
///         3 "rim" => { "hello rim!" }
///     }
/// );
///
/// // if user enter 2, then this assertion succeed:
/// assert_eq!(s, "hello rust!");
/// ```
///
/// **The user will see a prompt as below:**
///
/// ```console
/// choose a word to greet
///
/// 1) world
/// 2) rust
/// 3) rim
///
/// please enter your choice below [default: 1]
/// >
/// ```
macro_rules! handle_user_choice {
    ($ques:expr, $default:expr, { $($idx:literal $choice:expr => $action:block),+ }) => {
        {
            let choices__ = &[ $($choice),* ];
            let choice__ = $crate::cli::common::question_single_choice($ques, choices__, $default)?;
            match choice__ {
                $($idx => $action),*
                _ => unreachable!("`question_single_choice` ensures choice's range")
            }
        }
    };
}
pub(crate) use handle_user_choice;

use super::GlobalOpts;

/// A map representing a component's version difference related to it's name.
pub(crate) type VersionDiffMap<'c> = HashMap<&'c str, VersionDiff<'c>>;
/// A map contains the selected components with their indexes in the full component list.
///
/// Notice that this is an [`IndexMap`], which means the order will be preserved.
pub(crate) type ComponentChoices<'c> = IndexMap<usize, &'c Component>;

#[derive(Debug)]
pub(crate) struct VersionDiff<'c> {
    pub(crate) from: Option<&'c str>,
    pub(crate) to: Option<&'c str>,
    /// `true` a tool wasn't supported or installed previously, but have a new version
    /// available, which means that tool is newly supported.
    pub(crate) is_newly_supported: bool,
}

pub(crate) fn question_str<Q: Display, A: Display>(
    question: Q,
    extra: Option<&str>,
    default: A,
) -> Result<String> {
    question_str_with_retry(question, extra, None, default, |_| true)
}

pub(crate) fn question_str_with_retry<Q, A, F>(
    question: Q,
    extra: Option<&str>,
    // a prompt shown on top of user input, such as "please enter your choice below".
    prompt: Option<&str>,
    default: A,
    cond: F,
) -> Result<String>
where
    Q: Display,
    A: Display,
    F: Fn(&str) -> bool,
{
    let mut stdout = io::stdout();
    let default_badge = if !default.to_string().is_empty() {
        format!(" [{}: {default}]", t!("default"))
    } else {
        String::new()
    };
    // if there's a specified prompt or if the extra lines are too long,
    // we will display the default label above the actual input, making it more visible to users.
    let show_default_above_input =
        prompt.is_some() || extra.map(|e| e.lines().count() > 2).unwrap_or_default();

    // print question, with or without default label.
    if show_default_above_input {
        writeln!(&mut stdout, "{question}")?;
    } else {
        writeln!(&mut stdout, "{question}{default_badge}")?;
    }
    // print extra info, such as a list of selectable options.
    if let Some(ex) = extra {
        writeln!(&mut stdout, "\n{ex}")?;
    }

    loop {
        if let Some(prmt) = prompt {
            write!(&mut stdout, "{prmt} ")?;
        }
        if show_default_above_input {
            writeln!(&mut stdout, "{default_badge}")?;
        }
        write!(&mut stdout, "> ")?;
        _ = stdout.flush();

        let input_raw = readline()?;
        let input = input_raw.trim();
        writeln!(&mut stdout)?;

        if input.is_empty() {
            return Ok(default.to_string());
        } else if !cond(input) {
            continue;
        } else {
            return Ok(input.to_string());
        }
    }
}

/// Display a list of given `choices` and ask for user input that related to choice's index.
///
///
/// # Example
/// Each choice will be labelled with a number that equal to their **index + 1**, for example,
/// when given a choices list as:
///
/// ```ignore
/// let choices = &["do something", "do other things", "abort"];
/// let default = 1;
/// ```
///
/// It will be shown as:
///
/// ```console
/// 1) do something
/// 2) do other things
/// 3) abort
///
/// enter your choice below [default: 1]
/// >
/// ```
///
/// Therefore, if user enter "3", meaning they choose to "abort".
pub(crate) fn question_single_choice<Q, C, D>(
    question: Q,
    choices: &[C],
    default: D,
) -> Result<usize>
where
    Q: Display,
    C: Display,
    D: Display,
{
    let mut choices_prompt = String::new();

    for (idx, choice) in choices.iter().enumerate() {
        let choice_item = format!("{}) {choice}\n", idx + 1);
        choices_prompt.push_str(&choice_item);
    }

    let response = question_str_with_retry(
        question,
        Some(&choices_prompt),
        Some(t!("enter_choice_below").as_ref()),
        default,
        |s| match s.parse::<usize>() {
            Ok(u) if (1..=choices.len()).contains(&u) => true,
            _ => {
                let expected = t!(
                    "ranged_integer",
                    lower_bound = 1,
                    upper_bound = choices.len()
                );
                warn!("{}", t!("invalid_input", actual = s, expect = expected));
                false
            }
        },
    )?;
    Ok(response.parse()?)
}

/// Similar to [`question_single_choice`], but instead of asking user to type one integer,
/// this will ask for a list of integers that are separated by spaces.
pub(crate) fn question_multi_choices<Q, C, D>(
    question: Q,
    choices: &[C],
    default: D,
) -> Result<Vec<usize>>
where
    Q: Display,
    C: Display,
    D: Display,
{
    let mut choices_prompt = String::new();

    for (idx, choice) in choices.iter().enumerate() {
        let choice_item = format!("{}) {choice}\n", idx + 1);
        choices_prompt.push_str(&choice_item);
    }

    let response = question_str_with_retry(
        question,
        Some(&choices_prompt),
        Some(t!("enter_choice_below").as_ref()),
        default,
        |s| {
            if s.split_whitespace().all(
                |s| matches!(s.parse::<usize>(), Ok(idx) if (1..=choices.len()).contains(&idx)),
            ) {
                true
            } else {
                let expected = format!(
                    "{}{}",
                    t!("space_separated_and"),
                    t!(
                        "ranged_integer",
                        lower_bound = 1,
                        upper_bound = choices.len()
                    )
                );
                warn!("{}", t!("invalid_input", actual = s, expect = expected));
                false
            }
        },
    )?;

    Ok(response
        .split_whitespace()
        // The choices should already be valid at this point, but use filter_map just in case.
        .filter_map(|s| s.parse::<usize>().ok())
        .collect())
}

pub(crate) fn confirm<Q: Display>(question: Q, default: bool) -> Result<bool> {
    if GlobalOpts::get().yes_to_all {
        return Ok(true);
    }

    let mut stdout = io::stdout();
    writeln!(
        &mut stdout,
        "{} ({})",
        question,
        if default { "Y/n" } else { "y/N" }
    )?;
    write!(&mut stdout, "> ")?;
    _ = stdout.flush();

    let input = readline()?;
    let choice = match input.to_lowercase().as_str() {
        "y" | "yes" => true,
        "n" | "no" => false,
        "" => default,
        _ => false,
    };

    writeln!(&mut stdout)?;
    Ok(choice)
}

pub(crate) enum Confirm {
    Yes,
    No,
    Abort,
}

pub(crate) fn confirm_options() -> Result<Confirm> {
    let mut stdout = io::stdout();

    writeln!(&mut stdout, "\n{}\n", t!("question_options"))?;
    writeln!(&mut stdout, "1) {} ({})", t!("confirm"), t!("default"))?;
    writeln!(&mut stdout, "2) {}", t!("reenter"))?;
    writeln!(&mut stdout, "3) {}", t!("cancel"))?;
    write!(&mut stdout, "> ")?;
    _ = stdout.flush();

    let input = readline()?;
    let choice = match input.as_str() {
        "1" | "" => Confirm::Yes,
        "2" => Confirm::No,
        _ => Confirm::Abort,
    };

    writeln!(&mut stdout)?;
    Ok(choice)
}

pub(crate) fn show_confirmation(
    install_dir: Option<&str>,
    choices: &ComponentChoices<'_>,
    is_remove: bool,
) -> Result<()> {
    let mut stdout = std::io::stdout();

    writeln!(&mut stdout, "\n{}\n", t!("current_option"))?;
    if let Some(dir) = install_dir {
        writeln!(&mut stdout, "{}:\n\t{dir}", t!("install_dir"))?;
    }
    writeln!(
        &mut stdout,
        "\n{}:",
        if is_remove {
            t!("components_to_remove")
        } else {
            t!("components_to_install")
        }
    )?;
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
            for obsolete in comp.obsoletes() {
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

/// Create a collection of component choices base of a filtering condition.
/// Also taking component constrains, such as `requires`, `conflicts` into account.
// TODO: handle conflicts
pub(crate) fn component_choices_with_constrains<F>(
    all_components: &[Component],
    condition_callback: F,
) -> ComponentChoices<'_>
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
            let selected = condition_callback(*idx, c);
            if selected {
                dependencies.extend(c.dependencies());
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

/// Pausing the console window.
///
/// This will ask user to press `enter` key after the program finishes.
pub fn pause() -> Result<()> {
    if GlobalOpts::get().yes_to_all {
        return Ok(());
    }
    let mut stdout = io::stdout();
    write!(&mut stdout, "\n{}", t!("pause_prompt"))?;
    _ = stdout.flush();

    readline()?;
    Ok(())
}

fn readline() -> Result<String> {
    let mut input_buf = String::new();
    io::stdin()
        .lock()
        .read_line(&mut input_buf)
        .context("unable to read from standard input")?;
    Ok(input_buf.trim().to_string())
}

/// Specify the string after each component's name, which is usually wrapped in parentheses.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) enum ComponentDecoration<'c> {
    /// Version change info, i.e.:
    /// `hello-world (0.1.0 -> 0.2.0)`
    VersionDiff(&'c VersionDiffMap<'c>),
    /// Decorations to display during component selection, including `installed|required` tag
    /// to indicate whether a tool is installed or required but not installed,
    /// i.e.: `hello-world (required)`.
    Selection,
    /// A label to show during confirmation page, indicates whether a tool is installed but will be re-install,
    /// i.e.: `hello-world (installed, reinstalling)`
    Confirmation,
    /// No decoration string, this is the default variant, i.e.:
    /// `hello-world`
    #[default]
    None,
}

impl ComponentDecoration<'_> {
    pub(crate) fn for_component(self, comp: &Component) -> String {
        match self {
            Self::None => String::new(),
            Self::Selection => {
                if comp.installed {
                    format!(" ({})", t!("installed"))
                } else if comp.required {
                    format!(" ({})", t!("required"))
                } else {
                    String::new()
                }
            }
            Self::Confirmation => {
                if comp.installed {
                    format!(" ({})", t!("reinstall"))
                } else {
                    String::new()
                }
            }
            Self::VersionDiff(diff_map) => diff_map
                .get(comp.name.as_str())
                .map(|diff| {
                    format!(
                        " ({} -> {})",
                        diff.from.unwrap_or("N/A"),
                        diff.to.unwrap_or("N/A")
                    )
                })
                .unwrap_or_else(String::new),
        }
    }
}

/// A helper struct that convert [`Component`]s to
/// a list of component names with certain decoration.
pub(crate) struct ComponentListBuilder<'c, I> {
    components: I,
    show_desc: bool,
    decoration: ComponentDecoration<'c>,
}

impl<'c, I: IntoIterator<Item = &'c Component>> ComponentListBuilder<'c, I> {
    pub(crate) fn new(components: I) -> Self {
        Self {
            components,
            show_desc: false,
            decoration: ComponentDecoration::default(),
        }
    }

    pub(crate) fn show_desc(mut self, yes: bool) -> Self {
        self.show_desc = yes;
        self
    }

    pub(crate) fn decorate(mut self, deco: ComponentDecoration<'c>) -> Self {
        self.decoration = deco;
        self
    }

    pub(crate) fn build(self) -> Vec<String> {
        self.components
            .into_iter()
            .map(|c| {
                let deco = self.decoration.for_component(c);
                let desc = if self.show_desc {
                    if let Some(description) = &c.desc {
                        format!("\n\t{}: {description}", t!("description"))
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                format!("{}{deco}{desc}", &c.display_name)
            })
            .collect()
    }
}
