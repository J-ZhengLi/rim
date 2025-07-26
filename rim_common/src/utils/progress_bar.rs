//! Progress bar indicator for commandline user interface.

use anyhow::{Context, Result};
use indicatif::{ProgressBar as CliProgressBar, ProgressState, ProgressStyle as CliProgressStyle};
use std::time::Duration;

#[allow(unused_variables)]
/// Abstract progress sender/handler used for both CLI and GUI mode.
pub trait ProgressHandler: Send + Sync {
    /// Start the progress with a certain message.
    fn start(&self, msg: String) -> Result<()>;
    /// Update the progress to a value, or tick once if the value is `None`.
    fn update(&self, value: Option<u64>) -> Result<()>;
    /// Finish progress with a certain message.
    fn finish(&self, msg: String) -> Result<()>;
    /// Replace the progress bar with a certain style.
    fn set_style(&mut self, style: ProgressStyle) -> Result<()> {
        Ok(())
    }

    // Optional overall (master) progress control

    /// Start the master progress bar with a certain progress.
    fn start_master(&mut self, msg: String, length: Option<u64>) -> Result<()> {
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
pub enum ProgressStyle {
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

impl ProgressStyle {
    fn cli_pattern(&self) -> &str {
        match self {
            Self::Bytes(_) => "{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})",
            Self::Len(_) => "{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})",
            Self::Spinner{..} => "{spinner:.green} [{elapsed_precise}] {msg}",
            Self::Hidden => "",
        }
    }
}

/// Hidden progress indicator, nothing will be shown,
/// and all [`ProgressHandler`] methods do nothing.
#[derive(Debug, Clone)]
pub struct HiddenProgress;

impl ProgressHandler for HiddenProgress {
    fn start(&self, _msg: String) -> Result<()> {
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
    style: ProgressStyle,
}

impl Default for CliProgress {
    fn default() -> Self {
        Self {
            bar: CliProgressBar::hidden(),
            style: ProgressStyle::Hidden,
        }
    }
}

impl ProgressHandler for CliProgress {
    fn start(&self, msg: String) -> Result<()> {
        // log the starting of the progress
        info!("{msg}");
        self.bar.set_message(msg);
        self.bar.set_position(0);

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

    /// Replace the progress bar with a certain style.
    fn set_style(&mut self, style: ProgressStyle) -> Result<()> {
        let bar = match style {
            ProgressStyle::Bytes(len) | ProgressStyle::Len(len) => CliProgressBar::new(len),
            ProgressStyle::Spinner { auto_tick_duration } => {
                let bar = CliProgressBar::new_spinner();
                if let Some(interval) = auto_tick_duration {
                    bar.enable_steady_tick(interval);
                }
                bar
            }
            ProgressStyle::Hidden => CliProgressBar::hidden(),
        };

        bar.set_style(
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
        );
        self.bar = bar;
        self.style = style;

        Ok(())
    }
}
