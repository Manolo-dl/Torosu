//! What the desktop session allows, and telling the user something while
//! another window (osu!) has the focus.

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// Whether Torosu runs in a Wayland session, where an app can't listen to
/// the keyboard while another window has the focus: no global shortcuts.
#[tauri::command]
pub fn is_wayland() -> bool {
    crate::wayland_session()
}

/// Shows a notification of the system, seen over osu!.
#[tauri::command]
pub fn show_notification(app: AppHandle, title: String, body: String) {
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        log::error!("Failed to show notification: {}", e);
    }
}
