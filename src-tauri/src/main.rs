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
    torosu_lib::run()
}
