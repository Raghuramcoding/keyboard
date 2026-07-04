//! Session persistence: JSON files stored in the user's config directory.
//!
//! Path: `$XDG_CONFIG_HOME/tck/sessions/` (Linux, macOS) or
//!       `%APPDATA%/tck/sessions/` (Windows).

use std::fs;
use std::io;
use std::path::PathBuf;

/// A store for persisting session data as JSON files.
pub struct SessionStore {
    dir: PathBuf,
}

impl SessionStore {
    /// Create a new `SessionStore`, ensuring the sessions directory exists.
    ///
    /// The directory is created at `{config_dir}/tck/sessions/` where
    /// `config_dir` is provided by the `dirs` crate.
    pub fn new() -> io::Result<Self> {
        let dir = dirs::config_dir()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "config directory not found"))?
            .join("tck")
            .join("sessions");

        fs::create_dir_all(&dir)?;

        Ok(Self { dir })
    }

    /// Create a `SessionStore` rooted at an explicit path (useful for tests).
    pub fn with_path<P: Into<PathBuf>>(path: P) -> Self {
        Self { dir: path.into() }
    }

    /// Save session data under `id` as a JSON file.
    ///
    /// Writes to a `.tmp` file first, then atomically renames it to
    /// `{id}.json` so that partial writes are never observed.
    pub fn save(&self, id: &str, data: &impl serde::Serialize) -> io::Result<()> {
        let path = self.dir.join(format!("{id}.json"));
        let tmp_path = self.dir.join(format!("{id}.tmp"));

        let file = fs::File::create(&tmp_path)?;
        serde_json::to_writer(file, data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        fs::rename(&tmp_path, &path)?;
        Ok(())
    }

    /// Load session data previously saved under `id`.
    pub fn load<T: serde::de::DeserializeOwned>(&self, id: &str) -> io::Result<T> {
        let path = self.dir.join(format!("{id}.json"));
        let file = fs::File::open(&path)?;
        serde_json::from_reader(file).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// List all session IDs (the stem of each `.json` file in the store).
    pub fn list(&self) -> io::Result<Vec<String>> {
        let mut ids = Vec::new();

        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    // Skip temporary files (those ending in .tmp before rename).
                    if !stem.ends_with(".tmp") {
                        ids.push(stem.to_string());
                    }
                }
            }
        }

        Ok(ids)
    }

    /// Delete a session file by `id`.
    pub fn delete(&self, id: &str) -> io::Result<()> {
        let path = self.dir.join(format!("{id}.json"));
        fs::remove_file(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Session {
        label: String,
        active: bool,
    }

    #[test]
    fn round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::with_path(dir.path());

        let id = "test-round-trip";
        let data = Session {
            label: "my session".into(),
            active: true,
        };

        store.save(id, &data).unwrap();
        let loaded: Session = store.load(id).unwrap();
        assert_eq!(data, loaded);

        store.delete(id).unwrap();
    }

    #[test]
    fn list_ids() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::with_path(dir.path());

        store
            .save(
                "list-test-a",
                &Session {
                    label: "a".into(),
                    active: false,
                },
            )
            .unwrap();
        store
            .save(
                "list-test-b",
                &Session {
                    label: "b".into(),
                    active: true,
                },
            )
            .unwrap();

        let ids = store.list().unwrap();
        assert!(ids.contains(&"list-test-a".to_string()));
        assert!(ids.contains(&"list-test-b".to_string()));

        store.delete("list-test-a").unwrap();
        store.delete("list-test-b").unwrap();
    }

    #[test]
    fn load_missing() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::with_path(dir.path());
        let result: io::Result<Session> = store.load("non-existent");
        assert!(result.is_err());
    }
}
