use crate::{AppConfigState, commands::osu_db::folders::check_beatmapset_folder_exists};

#[tauri::command]
pub async fn validate_pack_folder(folder_name: String, osu_state: tauri::State<'_, AppConfigState>) -> Result<bool, String> {
    
    let osu_path = osu_state.config.lock().unwrap()
        .osu_path.clone()
        .ok_or("osu! path not set")?;

    let target_path = format!("{}/Songs/Custom-pack_{}", osu_path, sanitize(&folder_name));

    let exist = check_beatmapset_folder_exists(&target_path).await?;

    Ok(!exist)
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if "/\\:*?\"<>|".contains(c) { '_' } else { c })
        .collect()
}