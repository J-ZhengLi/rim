use std::sync::{Mutex, OnceLock};

use crate::{common::FrontendFunctionPayload, Result};
use rim::setter;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_positioner::{Position, WindowExt};

static CONTENT: OnceLock<Mutex<Notification>> = OnceLock::new();
// If adding more notification windows, make sure their label start with 'notification:'
pub(crate) const WINDOW_LABEL: &str = "notification:popup";

#[derive(Debug, Serialize, Clone)]
pub(crate) struct NotificationAction {
    pub(crate) label: String,
    pub(crate) icon: Option<String>,
    pub(crate) command: FrontendFunctionPayload,
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct Notification {
    title: String,
    content: String,
    actions: Vec<NotificationAction>,
    window_label: Option<String>,
}

impl Notification {
    pub(crate) fn new<T, C>(title: T, content: C, actions: Vec<NotificationAction>) -> Self
    where
        T: Into<String>,
        C: Into<String>,
    {
        let this = Self {
            title: title.into(),
            content: content.into(),
            actions,
            window_label: Some(WINDOW_LABEL.into()),
        };
        if let Some(existing) = CONTENT.get() {
            *existing.lock().unwrap() = this.clone();
        } else {
            CONTENT.set(Mutex::new(this.clone())).unwrap();
        }
        this
    }

    setter!(with_window_label(self.window_label, label: impl Into<String>) { Some(label.into()) });

    pub(crate) fn show(self, app_handle: &AppHandle) -> Result<()> {
        let label = self.window_label.as_deref().unwrap_or(WINDOW_LABEL);
        if let Some(popup) = app_handle.get_window(label) {
            popup.show()?;
            return Ok(());
        }

        let popup = tauri::WindowBuilder::new(
            app_handle,
            label,
            tauri::WindowUrl::App("notification.html".into()),
        )
        .always_on_top(true)
        .decorations(false)
        .resizable(false)
        .title("notification")
        .skip_taskbar(true)
        .inner_size(360.0, 220.0)
        .build()?;

        popup.move_window(Position::BottomRight)?;
        Ok(())
    }
}

#[tauri::command]
pub(crate) async fn notification_content() -> Option<Notification> {
    Some(CONTENT.get()?.lock().unwrap().clone())
}

#[tauri::command]
pub(crate) fn close(app: AppHandle, label: String) {
    let Some(window) = app.get_window(&label) else {
        return;
    };
    crate::common::close_window(window);
}

pub(crate) fn close_all_notification(app: AppHandle) {
    for (_, window) in app
        .windows()
        .iter()
        .filter(|(label, _)| label.starts_with("notification:"))
    {
        crate::common::close_window(window.clone());
    }
}
