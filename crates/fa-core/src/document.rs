//! An open document: where it lives, how it is encoded, and what we last saw
//! on disk. The text itself is owned by the editor.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{CoreError, Result};
use crate::fsio::{self, Fingerprint};
use crate::history::History;
use crate::text::{self, TextFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveMode {
    /// Refuse to save if the file changed on disk since we last read/wrote it.
    Safe,
    /// The user explicitly chose to replace the on-disk version.
    Overwrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiskState {
    Unchanged,
    Modified,
    Missing,
}

#[derive(Debug, Default)]
pub struct SaveOutcome {
    /// `false` when the bytes on disk were already identical.
    pub wrote: bool,
    /// Snapshotting the previous version failed; the save itself succeeded.
    pub history_warning: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Document {
    path: Option<PathBuf>,
    format: TextFormat,
    /// Fingerprint of the bytes we last read from / wrote to `path`.
    disk: Option<Fingerprint>,
}

impl Document {
    pub fn untitled() -> Self {
        Self {
            path: None,
            format: TextFormat::new_document(),
            disk: None,
        }
    }

    /// Open `path`, returning the document and its `\n`-normalised text.
    pub fn open(path: &Path) -> Result<(Self, String)> {
        let path = fsio::resolve_target(path)?;
        let bytes = fsio::read_file(&path)?;
        let (text, format) = text::decode(&bytes).map_err(|e| CoreError::NotUtf8 {
            offset: e.valid_up_to()
                + if bytes.starts_with(b"\xEF\xBB\xBF") {
                    3
                } else {
                    0
                },
            path: path.clone(),
        })?;
        let doc = Self {
            path: Some(path),
            format,
            disk: Some(Fingerprint::of(&bytes)),
        };
        Ok((doc, text))
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn format(&self) -> TextFormat {
        self.format
    }

    /// Compare what is on disk now with what we last saw.
    pub fn disk_state(&self) -> Result<DiskState> {
        let Some(path) = &self.path else {
            return Ok(DiskState::Unchanged);
        };
        Ok(match fsio::read_existing(path)? {
            None => DiskState::Missing,
            Some(bytes) if self.disk.as_ref() == Some(&Fingerprint::of(&bytes)) => {
                DiskState::Unchanged
            }
            Some(_) => DiskState::Modified,
        })
    }

    /// Re-read from disk, discarding what we knew.
    pub fn reload(&mut self) -> Result<String> {
        let path = self.path.clone().ok_or(CoreError::NoPath)?;
        let (doc, text) = Self::open(&path)?;
        *self = doc;
        Ok(text)
    }

    /// Save `text` to the document's path.
    pub fn save(
        &mut self,
        text: &str,
        mode: SaveMode,
        history: Option<&History>,
    ) -> Result<SaveOutcome> {
        let path = self.path.clone().ok_or(CoreError::NoPath)?;
        let current = fsio::read_existing(&path)?;

        if mode == SaveMode::Safe
            && let (Some(bytes), Some(known)) = (&current, &self.disk)
            && &Fingerprint::of(bytes) != known
        {
            return Err(CoreError::Conflict { path });
        }

        let bytes = text::encode(text, &self.format);
        let mut outcome = SaveOutcome::default();

        if current.as_deref() == Some(bytes.as_slice()) {
            self.disk = Some(Fingerprint::of(&bytes));
            return Ok(outcome);
        }

        if let (Some(previous), Some(history)) = (&current, history)
            && let Err(e) = history.snapshot(&path, previous)
        {
            outcome.history_warning = Some(e.to_string());
        }

        fsio::atomic_write(&path, &bytes)?;
        self.format.mixed_line_endings = false;
        self.disk = Some(Fingerprint::of(&bytes));
        outcome.wrote = true;
        Ok(outcome)
    }

    /// Save to a new location chosen by the user (who already confirmed any
    /// overwrite in the file dialog). On failure the document is unchanged.
    pub fn save_as(
        &mut self,
        path: &Path,
        text: &str,
        history: Option<&History>,
    ) -> Result<SaveOutcome> {
        let target = fsio::resolve_target(path)?;
        let previous = (self.path.replace(target), self.disk.take());
        let result = self.save(text, SaveMode::Overwrite, history);
        if result.is_err() {
            (self.path, self.disk) = previous;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn untouched_document_is_byte_identical_after_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.md");
        let original = b"\xEF\xBB\xBF# Hola\r\n\r\n* item\r\n";
        fs::write(&path, original).unwrap();

        let (mut doc, text) = Document::open(&path).unwrap();
        let outcome = doc.save(&text, SaveMode::Safe, None).unwrap();
        assert!(!outcome.wrote);
        assert_eq!(fs::read(&path).unwrap(), original);
    }

    #[test]
    fn external_change_is_detected_and_blocks_safe_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.md");
        fs::write(&path, "one\n").unwrap();
        let (mut doc, _) = Document::open(&path).unwrap();
        assert_eq!(doc.disk_state().unwrap(), DiskState::Unchanged);

        fs::write(&path, "changed elsewhere\n").unwrap();
        assert_eq!(doc.disk_state().unwrap(), DiskState::Modified);
        assert!(matches!(
            doc.save("mine\n", SaveMode::Safe, None),
            Err(CoreError::Conflict { .. })
        ));
        assert_eq!(fs::read_to_string(&path).unwrap(), "changed elsewhere\n");

        doc.save("mine\n", SaveMode::Overwrite, None).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "mine\n");
        assert_eq!(doc.disk_state().unwrap(), DiskState::Unchanged);
    }

    #[test]
    fn deleted_file_is_reported_and_can_be_recreated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.md");
        fs::write(&path, "x\n").unwrap();
        let (mut doc, _) = Document::open(&path).unwrap();
        fs::remove_file(&path).unwrap();
        assert_eq!(doc.disk_state().unwrap(), DiskState::Missing);
        doc.save("x\n", SaveMode::Safe, None).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn previous_version_goes_to_history() {
        let dir = tempfile::tempdir().unwrap();
        let history = History::new(dir.path().join("history"), 10);
        let path = dir.path().join("a.md");
        fs::write(&path, "v1\n").unwrap();
        let (mut doc, _) = Document::open(&path).unwrap();
        let outcome = doc.save("v2\n", SaveMode::Safe, Some(&history)).unwrap();
        assert!(outcome.wrote && outcome.history_warning.is_none());

        let snaps = history.list(doc.path().unwrap()).unwrap();
        assert_eq!(snaps.len(), 1);
        assert_eq!(fs::read(&snaps[0].file).unwrap(), b"v1\n");
    }

    #[test]
    fn save_as_failure_keeps_original_identity() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.md");
        fs::write(&path, "x\n").unwrap();
        let (mut doc, text) = Document::open(&path).unwrap();
        let bad = dir.path().join("missing-dir").join("b.md");
        assert!(doc.save_as(&bad, &text, None).is_err());
        assert_eq!(doc.path(), Some(fs::canonicalize(&path).unwrap().as_path()));
        assert_eq!(doc.disk_state().unwrap(), DiskState::Unchanged);
    }

    #[test]
    fn non_utf8_files_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("latin1.md");
        fs::write(&path, b"caf\xE9\n").unwrap();
        assert!(matches!(
            Document::open(&path),
            Err(CoreError::NotUtf8 { offset: 3, .. })
        ));
    }
}
