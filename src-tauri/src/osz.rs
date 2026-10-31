//! Writing `.osz` files: zip archives of a beatmap set.

use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use crate::error::Result;

/// An .osz being written. It is written to `<path>.part` and only gets its name
/// in [`OszWriter::finish`], so it is never seen half written: osu! picks up
/// any .osz in Songs, and a half-written one imports as a broken set. Dropped
/// before finishing (an error on the way), the `.part` is deleted.
pub struct OszWriter {
    zip: Option<ZipWriter<File>>,
    part: PathBuf,
    path: PathBuf,
}

impl OszWriter {
    pub fn create(path: &Path) -> Result<Self> {
        let mut part = path.as_os_str().to_os_string();
        part.push(".part");
        let part = PathBuf::from(part);

        let file = File::create(&part).map_err(|e| format!("{}: {}", part.display(), e))?;
        Ok(Self { zip: Some(ZipWriter::new(file)), part, path: path.to_path_buf() })
    }

    /// Adds the file at `source` as `name`, copied as it is read: audio and
    /// videos are not loaded whole.
    pub fn add_file(&mut self, name: &str, source: &Path) -> Result<()> {
        let mut file = File::open(source).map_err(|e| format!("{}: {}", source.display(), e))?;
        io::copy(&mut file, self.start(name)?)?;
        Ok(())
    }

    pub fn add_bytes(&mut self, name: &str, bytes: &[u8]) -> Result<()> {
        self.start(name)?.write_all(bytes)?;
        Ok(())
    }

    /// Starts the entry `name`; a name can only be used once.
    fn start(&mut self, name: &str) -> Result<&mut ZipWriter<File>> {
        let zip = self.zip.as_mut().expect("only `finish` takes the zip, and it consumes the writer");
        zip.start_file(name, SimpleFileOptions::default().compression_method(compression(name)))?;
        Ok(zip)
    }

    /// Completes the .osz and gives it its name.
    pub fn finish(mut self) -> Result<()> {
        if let Some(zip) = self.zip.take() {
            zip.finish()?;
        }
        fs::rename(&self.part, &self.path)?;
        Ok(())
    }
}

/// How an entry is kept: deflated if it is text or raw audio, which shrink a
/// lot (an .osu to a seventh, hitsound .wav files by a quarter). The audio,
/// images and videos are compressed already: deflating them would take time
/// to save 1 or 2%, so they are stored as they are.
fn compression(name: &str) -> CompressionMethod {
    let extension = name.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase());
    match extension.as_deref() {
        Some("osu" | "osb" | "wav" | "txt" | "bmp") => CompressionMethod::Deflated,
        _ => CompressionMethod::Stored,
    }
}

impl Drop for OszWriter {
    fn drop(&mut self) {
        // The file is closed before it is deleted, which Windows needs. After
        // `finish` there is no `.part` left to delete.
        drop(self.zip.take());
        let _ = fs::remove_file(&self.part);
    }
}

/// Opens an .osz in osu!, which imports it.
///
/// On Linux osu! runs under Wine, and xdg-open usually takes the .osz for the
/// zip it is, so it lands in the archive manager without an error. There
/// osu-winello's own handler is tried first, then `gio open`, which tells an
/// .osz apart by its extension when a handler registered it, and only then
/// the default app, as everywhere else.
pub fn open(osz_path: &Path) -> Result<()> {
    #[cfg(target_os = "linux")]
    if open_under_wine(osz_path) {
        return Ok(());
    }

    log::debug!("opening osz with the default app: {:?}", osz_path);
    tauri_plugin_opener::open_path(osz_path, None::<&str>).map_err(|e| {
        log::error!("failed to open {:?}: {}", osz_path, e);
        format!("Failed to open .osz: {}", e).into()
    })
}

/// Whether the .osz went to osu! under Wine: through osu-winello, or through
/// the app `gio` has for it.
#[cfg(target_os = "linux")]
fn open_under_wine(osz_path: &Path) -> bool {
    use std::process::{Command, Stdio};

    if let Some(osu_wine) = osu_winello() {
        log::debug!("opening {:?} with {:?} --osuhandler", osz_path, osu_wine);
        match Command::new(&osu_wine).arg("--osuhandler").arg(osz_path).stdin(Stdio::null()).spawn() {
            Ok(mut child) => {
                // It can run for a while (it may start osu!): waited for aside,
                // so it does not stay a zombie.
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
                return true;
            }
            Err(e) => log::warn!("{:?} --osuhandler failed: {}", osu_wine, e),
        }
    }

    match Command::new("gio").arg("open").arg(osz_path).stdin(Stdio::null()).status() {
        Ok(status) if status.success() => {
            log::debug!("opened {:?} with gio open", osz_path);
            true
        }
        Ok(status) => {
            log::warn!("gio open {:?} failed: {}", osz_path, status);
            false
        }
        Err(e) => {
            log::debug!("gio is not available: {}", e);
            false
        }
    }
}

/// osu-winello's launcher, `osu-wine`, if it has `--osuhandler`, which hands
/// a file to the osu! it installed. Looked for in the PATH and in
/// ~/.local/bin, where osu-winello puts it: an app started from the desktop
/// does not always have that folder in its PATH.
#[cfg(target_os = "linux")]
fn osu_winello() -> Option<PathBuf> {
    let path = std::env::var_os("PATH").map(|path| std::env::split_paths(&path).collect::<Vec<_>>()).unwrap_or_default();
    let local_bin = std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/bin"));

    path.into_iter()
        .chain(local_bin)
        .map(|dir| dir.join("osu-wine"))
        .find(|launcher| fs::read_to_string(launcher).is_ok_and(|script| script.contains("--osuhandler")))
}

/// Every file inside `folder`, subfolders included, as `(name, path)`: `name`
/// is its path from `folder` with `/` separators, as inside an .osz.
pub fn files_in(folder: &Path) -> Result<Vec<(String, PathBuf)>> {
    let mut files = Vec::new();
    let mut pending = vec![folder.to_path_buf()];

    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).map_err(|e| format!("{}: {}", dir.display(), e))? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }

            let Ok(relative) = path.strip_prefix(folder) else { continue };
            files.push((relative.to_string_lossy().replace('\\', "/"), path));
        }
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;
    use crate::test_utils::TestDir;

    fn entries(path: &Path) -> Vec<(String, String)> {
        let mut zip = zip::ZipArchive::new(File::open(path).unwrap()).unwrap();
        (0..zip.len())
            .map(|i| {
                let mut file = zip.by_index(i).unwrap();
                let mut content = String::new();
                file.read_to_string(&mut content).unwrap();
                (file.name().to_string(), content)
            })
            .collect()
    }

    #[test]
    fn writes_the_files_and_names_the_osz_once_finished() {
        let dir = TestDir::new("osz-finish");
        dir.write("audio.mp3", "audio");
        let path = dir.0.join("set.osz");

        let mut osz = OszWriter::create(&path).unwrap();
        osz.add_file("audio.mp3", &dir.0.join("audio.mp3")).unwrap();
        osz.add_bytes("diff.osu", b"osu file").unwrap();
        assert!(!path.exists());
        osz.finish().unwrap();

        let expected = [("audio.mp3".to_string(), "audio".to_string()), ("diff.osu".to_string(), "osu file".to_string())];
        assert_eq!(entries(&path), expected);
        assert_eq!(dir.read("set.osz.part"), None);
    }

    #[test]
    fn deflates_text_and_stores_media() {
        let dir = TestDir::new("osz-compression");
        let path = dir.0.join("set.osz");
        let osu = "0,0,1000,1,0\n".repeat(1000);

        let mut osz = OszWriter::create(&path).unwrap();
        osz.add_bytes("audio.ogg", b"audio").unwrap();
        osz.add_bytes("Diff [Insane].OSU", osu.as_bytes()).unwrap();
        osz.finish().unwrap();

        let mut zip = zip::ZipArchive::new(File::open(&path).unwrap()).unwrap();
        assert_eq!(zip.by_name("audio.ogg").unwrap().compression(), CompressionMethod::Stored);
        let diff = zip.by_name("Diff [Insane].OSU").unwrap();
        assert_eq!(diff.compression(), CompressionMethod::Deflated);
        assert!(diff.compressed_size() < diff.size() / 10);
        drop(diff);
        assert_eq!(entries(&path)[1].1, osu);
    }

    #[test]
    fn leaves_nothing_behind_when_not_finished() {
        let dir = TestDir::new("osz-drop");
        let path = dir.0.join("set.osz");

        let mut osz = OszWriter::create(&path).unwrap();
        osz.add_bytes("diff.osu", b"osu file").unwrap();
        assert!(osz.add_bytes("diff.osu", b"again").is_err(), "a name can only be used once");
        drop(osz);

        assert!(!path.exists());
        assert_eq!(dir.read("set.osz.part"), None);
    }

    #[test]
    fn lists_the_files_of_subfolders_with_slashes() {
        let dir = TestDir::new("osz-files");
        dir.write("bg.jpg", "bg");
        fs::create_dir_all(dir.0.join("sb/sprites")).unwrap();
        dir.write("sb/sprites/star.png", "star");

        let mut names: Vec<String> = files_in(&dir.0).unwrap().into_iter().map(|(name, _)| name).collect();
        names.sort();
        assert_eq!(names, ["bg.jpg", "sb/sprites/star.png"]);
    }
}
