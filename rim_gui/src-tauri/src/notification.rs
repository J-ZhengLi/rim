use std::{
    collections::VecDeque,
    sync::{LazyLock, Mutex},
};

use crate::{common::FrontendFunctionPayload, Result};
use rim::setter;
use serde::Serialize;
use tauri::{AppHandle, Manager, PhysicalPosition};

/// The y-axis offset of each notification window.
///
/// The first notification will have an offset of 0,
/// and the second notification will have an offset of `0 + WINDOW_HEIGHT`,
/// so that new notification will always appear above the previous one.
static WINDOW_POS_Y_OFFSET: Mutex<f64> = Mutex::new(0.0);
/// A global collection of notification contents.
///
/// Once a new notification is created, its content will be pushed into this queue,
/// and then a new window will be created once the `show()` method was called.
/// Then after the windows complete the setup process, it invoke a certain command
/// here and loads one content from here in a FIFO order.
static CONTENT_QUEUE: LazyLock<Mutex<VecDeque<Notification>>> =
    LazyLock::new(|| Mutex::new(VecDeque::new()));

// If adding more notification windows, make sure their label start with 'notification:'
pub(crate) const WINDOW_LABEL: &str = "notification:popup";

#[derive(Debug, Serialize, Clone)]
pub(crate) struct NotificationAction {
    pub(crate) label: String,
    pub(crate) icon: Option<String>,
    pub(crate) command: FrontendFunctionPayload,
}

/// A customized notification, acting as a separated tauri window.
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
        Self {
            title: title.into(),
            content: content.into(),
            actions,
            window_label: Some(WINDOW_LABEL.into()),
        }
    }

    setter!(with_window_label(self.window_label, label: impl Into<String>) { Some(label.into()) });

    /// Show notification as a separated window at the bottom right of the screen.
    ///
    /// Note: Although it's preferrable to use OS native notification system,
    /// such as relying on third-party crate such as [`notify-rust`].
    /// However, it doesn't seem like any of those crates supports
    /// custom actions on Windows and MacOS yet. If that feature ever became available on
    /// Windows and MacOS, this can be adjusted to show native notification instead.
    pub(crate) fn show(self, app_handle: &AppHandle) -> Result<()> {
        use crate::consts::{NOTIFICATION_WINDOW_HEIGHT, NOTIFICATION_WINDOW_WIDTH};

        let label = self
            .window_label
            .as_deref()
            .unwrap_or(WINDOW_LABEL)
            .to_string();
        if let Some(popup) = app_handle.get_window(&label) {
            popup.show()?;
            return Ok(());
        }

        // this will be a new notification, push the content to the queue.
        let mut guard = CONTENT_QUEUE.lock().unwrap();
        guard.push_back(self);

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
        .inner_size(NOTIFICATION_WINDOW_WIDTH, NOTIFICATION_WINDOW_HEIGHT)
        // show the window after we move it, to prevent flashing white window for a split sec.
        .visible(false)
        .build()?;

        if let Some(monitor) = popup.current_monitor()? {
            // place the notification window at the bottom right corner
            let monitor_size = monitor.size();
            let monitor_pos = monitor.position();
            let window_size = popup.outer_size()?;
            let mut pos_y_offset = WINDOW_POS_Y_OFFSET.lock().unwrap();
            let target_pos = PhysicalPosition::new(
                monitor_pos.x + monitor_size.width as i32 - window_size.width as i32,
                monitor_pos.y + monitor_size.height as i32
                    - window_size.height as i32
                    - *pos_y_offset as i32,
            );

            popup.set_position(target_pos)?;
            popup.show()?;

            *pos_y_offset += window_size.height as f64;
        }

        Ok(())
    }
}

#[tauri::command]
pub(crate) async fn notification_content() -> Option<Notification> {
    CONTENT_QUEUE.lock().unwrap().pop_front()
}

#[tauri::command]
pub(crate) fn close(app: AppHandle, label: String) {
    let Some(window) = app.get_window(&label) else {
        return;
    };
    crate::common::close_window(window);
    *WINDOW_POS_Y_OFFSET.lock().unwrap() -= crate::consts::NOTIFICATION_WINDOW_HEIGHT;
}

pub(crate) fn close_all_notification(app: AppHandle) {
    for (_, window) in app
        .windows()
        .iter()
        .filter(|(label, _)| label.starts_with("notification:"))
    {
        crate::common::close_window(window.clone());
    }
    *WINDOW_POS_Y_OFFSET.lock().unwrap() = 0.0;
}
