use osu_memory_reader::OsuReader;

/// Whether osu! stable (`osu!.exe`, also under Wine) is running. Only lists
/// the processes: no memory is read, so it needs no permissions.
pub fn osu_running() -> bool {
    !OsuReader::find_processes().is_empty()
}

/// `osu_running`, for the pages that must wait for osu! to be closed.
#[tauri::command]
pub fn is_osu_running() -> bool {
    osu_running()
}
