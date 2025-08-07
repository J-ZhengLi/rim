//! Progress bar indicator for commandline user interface.

use anyhow::{Context, Result};
use indicatif::{ProgressBar as CliProgressBar, ProgressState, ProgressStyle as CliProgressStyle};
use std::time::Duration;

#[allow(unused_variables)]
/// Abstract progress sender/handler used for both CLI and GUI mode.
pub trait ProgressHandler: Send + Sync {
    /// Start the progress with a certain message and style.
    fn start(&mut self, msg: String, style: ProgressKind) -> Result<()>;
    /// Update the progress to a value, or tick once if the value is `None`.
    fn update(&self, value: Option<u64>) -> Result<()>;
    /// Finish progress with a certain message.
    fn finish(&self, msg: String) -> Result<()>;

    // Optional overall (master) progress control

    /// Start the master progress bar with a certain progress.
    fn start_master(&mut self, msg: String, style: ProgressKind) -> Result<()> {
        Ok(())
    }
    /// Update the master progress bar, or tick once if the value is `None`.
    fn update_master(&self, value: Option<u64>) -> Result<()> {
        Ok(())
    }
    /// Finish the master progress with a certain massage.
    fn finish_master(&self, msg: String) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ProgressKind {
    /// Display the progress base on number of bytes.
    Bytes(u64),
    /// Display the progress base on position & length parameters.
    Len(u64),
    /// A spinner that spins as the progress goes, this does not require
    /// length information.
    Spinner {
        /// Set continuous spinning for a given amount of time.
        auto_tick_duration: Option<Duration>,
    },
    /// Show no progress bar at all.
    Hidden,
}

impl ProgressKind {
    fn cli_pattern(&self) -> &str {
        match self {
            Self::Bytes(_) => "{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})",
            Self::Len(_) => "{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})",
            Self::Spinner{..} => "{spinner:.green} [{elapsed_precise}] {msg}",
            Self::Hidden => "",
        }
    }

    pub fn length(&self) -> Option<u64> {
        match self {
            Self::Bytes(len) | Self::Len(len) => Some(*len),
            _ => None,
        }
    }
}

/// Hidden progress indicator, nothing will be shown,
/// and all [`ProgressHandler`] methods do nothing.
#[derive(Debug, Clone)]
pub struct HiddenProgress;

impl ProgressHandler for HiddenProgress {
    fn start(&mut self, _msg: String, _style: ProgressKind) -> Result<()> {
        Ok(())
    }
    fn finish(&self, _msg: String) -> Result<()> {
        Ok(())
    }
    fn update(&self, _value: Option<u64>) -> Result<()> {
        Ok(())
    }
}

/// Progress indicator for CLI
#[derive(Debug, Clone)]
pub struct CliProgress {
    bar: CliProgressBar,
    style: ProgressKind,
}

impl Default for CliProgress {
    fn default() -> Self {
        Self {
            bar: CliProgressBar::hidden(),
            style: ProgressKind::Hidden,
        }
    }
}

impl ProgressHandler for CliProgress {
    fn start(&mut self, msg: String, style: ProgressKind) -> Result<()> {
        // log the starting of the progress
        info!("{msg}");

        let bar = match style {
            ProgressKind::Bytes(len) | ProgressKind::Len(len) => CliProgressBar::new(len),
            ProgressKind::Spinner { auto_tick_duration } => {
                let bar = CliProgressBar::new_spinner();
                if let Some(interval) = auto_tick_duration {
                    bar.enable_steady_tick(interval);
                }
                bar
            }
            ProgressKind::Hidden => CliProgressBar::hidden(),
        };

        self.bar = bar
            .with_style(
                CliProgressStyle::with_template(style.cli_pattern())
                    .with_context(|| {
                        format!("Internal error: Invalid style pattern defined for {style:?}")
                    })?
                    .with_key(
                        "eta",
                        |state: &ProgressState, w: &mut dyn std::fmt::Write| {
                            write!(w, "{:.1}s", state.eta().as_secs_f64())
                                .expect("unable to display progress bar")
                        },
                    )
                    .progress_chars("#>-"),
            )
            .with_message(msg)
            .with_position(0);
        self.style = style;

        Ok(())
    }

    fn update(&self, value: Option<u64>) -> Result<()> {
        if let Some(val) = value {
            self.bar.set_position(val);
        } else {
            self.bar.tick();
        }

        Ok(())
    }

    fn finish(&self, msg: String) -> Result<()> {
        self.bar.finish_with_message(msg.clone());
        // log the starting of the progress.
        // NB: This need to be done after `finish_with_message` to prevent
        // showing double progress bar on terminal
        info!("{msg}");
        Ok(())
    }
}
