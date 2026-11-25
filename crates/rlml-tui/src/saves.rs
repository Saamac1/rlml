//! Saves in a folder (M 5.2, M 5.4): one `NAME.save` file a slot. A save
//! is written to `NAME.save.tmp` and renamed over the old one, so a crash
//! in the middle leaves the old save whole.

use std::path::{Path, PathBuf};

use rlml_core::rt::store::Store;

pub struct DiskStore {
    dir: PathBuf,
}

impl DiskStore {
    pub fn new(dir: &Path) -> DiskStore {
        DiskStore { dir: dir.to_path_buf() }
    }

    fn file(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{name}.save"))
    }
}

impl Store for DiskStore {
    fn read(&self, name: &str) -> Option<String> {
        std::fs::read_to_string(self.file(name)).ok()
    }

    fn write(&mut self, name: &str, text: &str) -> Result<(), String> {
        let err = |e: std::io::Error| format!("{}: {e}", self.dir.display());
        std::fs::create_dir_all(&self.dir).map_err(err)?;
        let tmp = self.dir.join(format!("{name}.save.tmp"));
        std::fs::write(&tmp, text).map_err(err)?;
        std::fs::rename(&tmp, self.file(name)).map_err(err)
    }

    fn remove(&mut self, name: &str) {
        let _ = std::fs::remove_file(self.file(name));
    }

    fn list(&self) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(&self.dir) else { return vec![] };
        let mut out: Vec<String> = entries.flatten().filter_map(|e| e.file_name().to_str()?.strip_suffix(".save").map(String::from)).collect();
        out.sort();
        out
    }
}

/// The player's data folder for a game (D 3.2): the system's user data
/// folder, then `rlml/GAME`.
pub fn user_data(game: &str) -> Option<PathBuf> {
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    let base = if cfg!(target_os = "windows") {
        env("APPDATA")
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library/Application Support"))
    } else {
        env("XDG_DATA_HOME").or_else(|| env("HOME").map(|h| h.join(".local/share")))
    }?;
    Some(base.join("rlml").join(game))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_keeps_saves_and_lists_them() {
        let dir = std::env::temp_dir().join(format!("rlml-saves-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut s = DiskStore::new(&dir);
        assert!(s.list().is_empty(), "no folder yet");
        s.write("auto-1", "one").unwrap();
        s.write("manual-1", "two").unwrap();
        s.write("auto-1", "three").unwrap();
        assert_eq!(s.list(), ["auto-1", "manual-1"]);
        assert_eq!(s.read("auto-1").as_deref(), Some("three"), "the write replaced the save");
        assert!(!dir.join("auto-1.save.tmp").exists(), "the temporary file was renamed");
        // a write that broke off leaves its temporary file, which is no save
        std::fs::write(dir.join("quit.save.tmp"), "half").unwrap();
        assert_eq!(s.list(), ["auto-1", "manual-1"]);
        s.remove("auto-1");
        assert_eq!(s.list(), ["manual-1"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
