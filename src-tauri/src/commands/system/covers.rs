//! Small covers of the beatmap sets, for their cards: from the background of
//! the set's own files, so they work offline, for maps not on the website and
//! with no request to anyone. A background is a large image (often 1920 px,
//! a few MB): it is made into a small JPEG once, kept in the app's cache, and
//! read from there afterwards.

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    io::Cursor,
    path::{Path, PathBuf},
    time::SystemTime,
};

use image::{codecs::jpeg::JpegEncoder, imageops::FilterType};
use tauri::{ipc::Response, AppHandle, Manager};

use crate::{commands::packs::creator::background_filename, error::Result, paths};

/// The width of a cover: twice what a card shows it at, for sharp screens.
const COVER_WIDTH: u32 = 480;
/// And at most this tall: a portrait background is cropped to its middle.
const COVER_MAX_HEIGHT: u32 = 270;
const COVER_QUALITY: u8 = 78;
/// Changed when the covers change, so the old ones aren't used.
const COVER_VERSION: u32 = 1;

/// The cover of the set in `folder` (of Songs), from the background its
/// `.osu` file `osu_file` names, as a JPEG. An error if it has none.
#[tauri::command]
pub async fn beatmap_cover(app: AppHandle, folder: String, osu_file: String) -> Result<Response> {
    let songs = tokio::fs::canonicalize(paths::songs_dir(&app)?)
        .await
        .map_err(|e| format!("Songs folder not found: {e}"))?;
    let cache = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("no cache folder: {e}"))?
        .join("covers");

    // Decoding and scaling an image takes a while: not on the async runtime.
    let bytes = tokio::task::spawn_blocking(move || cover(&songs, &cache, &folder, &osu_file))
        .await
        .map_err(|e| format!("cover task failed: {e}"))??;
    Ok(Response::new(bytes))
}

fn cover(songs: &Path, cache: &Path, folder: &str, osu_file: &str) -> Result<Vec<u8>> {
    let set = inside(songs, &songs.join(folder))?;
    let osu = inside(songs, &set.join(osu_file))?;
    let content = std::fs::read_to_string(&osu).map_err(|e| format!("can't read {osu:?}: {e}"))?;
    let name = background_filename(&content).ok_or("the map has no background")?;
    let background = inside(songs, &set.join(name.replace('\\', "/")))?;

    // Made again only if the background changed after it.
    let cached = cache.join(cache_name(&background));
    if modified(&cached).is_some_and(|made| modified(&background).is_none_or(|changed| changed <= made)) {
        if let Ok(bytes) = std::fs::read(&cached) {
            return Ok(bytes);
        }
    }

    let bytes = make_cover(&std::fs::read(&background).map_err(|e| format!("can't read {background:?}: {e}"))?)?;
    if std::fs::create_dir_all(cache).is_ok() {
        if let Err(e) = std::fs::write(&cached, &bytes) {
            log::warn!("could not keep the cover {cached:?}: {e}");
        }
    }
    Ok(bytes)
}

/// `path`, resolved, if it is inside the Songs folder: the folder and file
/// names come from the frontend and from .osu files, and `..` or a symlink
/// must not read anything else.
fn inside(songs: &Path, path: &Path) -> Result<PathBuf> {
    let resolved = std::fs::canonicalize(path).map_err(|e| format!("{path:?} not found: {e}"))?;
    if resolved.starts_with(songs) {
        Ok(resolved)
    } else {
        Err("path is outside the Songs folder".into())
    }
}

/// A file name for the cover of `background`, the same each time.
fn cache_name(background: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    COVER_VERSION.hash(&mut hasher);
    background.hash(&mut hasher);
    format!("{:016x}.jpg", hasher.finish())
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// The background, `COVER_WIDTH` wide (never enlarged) and at most
/// `COVER_MAX_HEIGHT` tall, as a JPEG.
fn make_cover(background: &[u8]) -> Result<Vec<u8>> {
    let image = image::load_from_memory(background).map_err(|e| format!("can't decode the background: {e}"))?;
    let width = image.width().min(COVER_WIDTH).max(1);
    let height = ((u64::from(image.height()) * u64::from(width)) / u64::from(image.width().max(1))).max(1) as u32;
    let mut small = image.resize_exact(width, height, FilterType::Triangle);
    if small.height() > COVER_MAX_HEIGHT {
        let top = (small.height() - COVER_MAX_HEIGHT) / 2;
        small = small.crop_imm(0, top, small.width(), COVER_MAX_HEIGHT);
    }

    let mut out = Cursor::new(Vec::new());
    JpegEncoder::new_with_quality(&mut out, COVER_QUALITY)
        .encode_image(&small.to_rgb8())
        .map_err(|e| format!("can't encode the cover: {e}"))?;
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        image::RgbImage::from_pixel(width, height, image::Rgb([200, 40, 90]))
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        bytes
    }

    #[test]
    fn scales_a_wide_background_down_to_the_cover_width() {
        let cover = image::load_from_memory(&make_cover(&png(1920, 1080)).unwrap()).unwrap();
        assert_eq!((cover.width(), cover.height()), (480, 270));
    }

    #[test]
    fn crops_a_tall_background_and_never_enlarges() {
        let tall = image::load_from_memory(&make_cover(&png(480, 900)).unwrap()).unwrap();
        assert_eq!((tall.width(), tall.height()), (480, 270));
        let small = image::load_from_memory(&make_cover(&png(320, 180)).unwrap()).unwrap();
        assert_eq!((small.width(), small.height()), (320, 180));
    }

    #[test]
    fn makes_and_keeps_a_cover_from_the_set_background() {
        let root = std::env::temp_dir().join(format!("torosu-covers-{}", std::process::id()));
        let songs = root.join("Songs");
        let set = songs.join("1 Artist - Title");
        std::fs::create_dir_all(&set).unwrap();
        std::fs::write(set.join("bg.png"), png(1280, 720)).unwrap();
        std::fs::write(set.join("map.osu"), "osu file format v14\n\n[Events]\n0,0,\"bg.png\",0,0\n").unwrap();
        let songs = std::fs::canonicalize(&songs).unwrap();
        let cache = root.join("cache");

        let first = cover(&songs, &cache, "1 Artist - Title", "map.osu").unwrap();
        assert_eq!(std::fs::read_dir(&cache).unwrap().count(), 1);
        assert_eq!(cover(&songs, &cache, "1 Artist - Title", "map.osu").unwrap(), first);
        assert!(cover(&songs, &cache, "../..", "map.osu").is_err());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
