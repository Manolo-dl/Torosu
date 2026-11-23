// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Before any thread: large blocks given back when freed (see allocator.rs).
    torosu_lib::allocator::give_memory_back();

    // WebKitGTK's DMABUF renderer left the window blank on Wayland with
    // NVIDIA's driver before it had explicit sync (555). Turned off only
    // there: off, WebKit draws in software and animations stutter. A value
    // set by hand is kept, to try either way.
    if torosu_lib::wayland_session()
        && std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none()
        && torosu_lib::nvidia_without_explicit_sync()
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    // The AppImage brings Ubuntu 22.04's WebKitGTK (2.50), patched to turn
    // its DMABUF renderer off whenever the GL vendor is NVIDIA: with it every
    // GPU path, so the page was drawn on the CPU and animations stuttered.
    // WEBKIT_FORCE_DMABUF_RENDERER skips that check (drivers from 555 on
    // handle the renderer); then its GTK fails with "Error 71 (Protocol
    // error)" on Wayland with the driver's explicit sync, so that is turned
    // off for Torosu. Values set by hand are kept.
    if torosu_lib::wayland_session()
        && std::env::var_os("APPIMAGE").is_some()
        && torosu_lib::nvidia_with_explicit_sync()
    {
        for (name, value) in [("WEBKIT_FORCE_DMABUF_RENDERER", "1"), ("__NV_DISABLE_EXPLICIT_SYNC", "1")] {
            if std::env::var_os(name).is_none() {
                std::env::set_var(name, value);
            }
        }
    }
    torosu_lib::run()
}
