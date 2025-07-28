//! GUI progress bar module

use rim_common::utils::{ProgressHandler, ProgressStyle};
use serde::Serialize;
use tauri::{AppHandle, Manager};

const MAIN_PROGRESS_START_EVENT: &str = "progress:main-start";
const MAIN_PROGRESS_UPDATE_EVENT: &str = "progress:main-update";
const MAIN_PROGRESS_END_EVENT: &str = "progress:main-end";
const SUB_PROGRESS_START_EVENT: &str = "progress:sub-start";
const SUB_PROGRESS_UPDATE_EVENT: &str = "progress:sub-update";
const SUB_PROGRESS_END_EVENT: &str = "progress:sub-end";

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
enum GuiProgressStyle {
    Bytes,
    Len,
    Spinner,
    Hidden,
}

impl From<ProgressStyle> for GuiProgressStyle {
    fn from(value: ProgressStyle) -> Self {
        match value {
            ProgressStyle::Bytes(_) => GuiProgressStyle::Bytes,
            ProgressStyle::Len(_) => GuiProgressStyle::Len,
            ProgressStyle::Spinner { .. } => GuiProgressStyle::Spinner,
            ProgressStyle::Hidden => GuiProgressStyle::Hidden,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ProgressPayload {
    message: String,
    style: GuiProgressStyle,
    length: Option<u64>,
}

#[derive(Debug, Clone)]
pub(crate) struct GuiProgress {
    handle: AppHandle,
}

impl GuiProgress {
    pub(crate) fn new(handle: AppHandle) -> Self {
        Self {
            handle,
        }
    }
}

impl ProgressHandler for GuiProgress {
    fn start(&mut self, msg: String, style: ProgressStyle) -> anyhow::Result<()> {
        let (length, gui_style) = match style {
            ProgressStyle::Bytes(len) => (Some(len), GuiProgressStyle::Bytes),
            ProgressStyle::Len(len) => (Some(len), GuiProgressStyle::Len),
            ProgressStyle::Spinner { .. } => (None, GuiProgressStyle::Spinner),
            ProgressStyle::Hidden => (None, GuiProgressStyle::Hidden),
        };

        let payload = ProgressPayload {
            message: msg,
            length,
            style: gui_style,
        };

        self.handle.emit_all(SUB_PROGRESS_START_EVENT, payload)?;
        Ok(())
    }

    fn update(&self, value: Option<u64>) -> anyhow::Result<()> {
        self.handle.emit_all(SUB_PROGRESS_UPDATE_EVENT, value)?;
        Ok(())
    }

    fn finish(&self, msg: String) -> anyhow::Result<()> {
        self.handle.emit_all(SUB_PROGRESS_END_EVENT, msg)?;
        Ok(())
    }

    fn start_master(&mut self, msg: String, style: ProgressStyle) -> anyhow::Result<()> {
        let payload = ProgressPayload {
            message: msg,
            length: style.length(),
            style: style.into(),
        };

        self.handle.emit_all(MAIN_PROGRESS_START_EVENT, payload)?;
        Ok(())
    }

    fn update_master(&self, value: Option<u64>) -> anyhow::Result<()> {
        self.handle.emit_all(MAIN_PROGRESS_UPDATE_EVENT, value)?;
        Ok(())
    }

    fn finish_master(&self, msg: String) -> anyhow::Result<()> {
        self.handle.emit_all(MAIN_PROGRESS_END_EVENT, msg)?;
        Ok(())
    }
}
