//! Writing `.osz` files: zip archives of a beatmap set, stored without
//! compression as osu! does.

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
        zip.start_file(name, SimpleFileOptions::default().compression_method(CompressionMethod::Stored))?;
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

impl Drop for OszWriter {
    fn drop(&mut self) {
        // The file is closed before it is deleted, which Windows needs. After
        // `finish` there is no `.part` left to delete.
        drop(self.zip.take());
        let _ = fs::remove_file(&self.part);
    }
}

/// Opens an .osz with what the system opens it with, osu! itself, which
/// imports it. On Linux osu! often runs under osu-wine, without a file
/// association: that is tried next.
pub fn open(osz_path: &Path) -> Result<()> {
    log::debug!("opening osz with default handler: {:?}", osz_path);

    if let Err(e) = tauri_plugin_opener::open_path(osz_path, None::<&str>) {
        log::warn!("default handler failed to open .osz ({}), trying osu-wine --osuhandler", e);

        let osu_wine_result = std::process::Command::new("osu-wine")
            .arg("--osuhandler")
            .arg(osz_path)
            .spawn();

        match osu_wine_result {
            Ok(_) => {
                log::debug!("opened .osz via osu-wine --osuhandler");
            }
            Err(wine_err) => {
                log::error!("failed to open .osz with both default handler and osu-wine: {} / {}", e, wine_err);
                return Err(format!("Failed to open .osz: {}", e).into());
            }
        }
    } else {
        log::debug!("opened .osz with default handler");
    }

    Ok(())
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
