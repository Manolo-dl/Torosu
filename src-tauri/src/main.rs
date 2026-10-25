// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    {
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            //std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }
    }
    torosu_lib::run()
}
