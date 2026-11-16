//! Local version history: before a document is overwritten, its previous
//! on-disk contents are copied into a private, per-user folder.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::error::{CoreError, Result};
use crate::fsio;

const SNAPSHOT_EXT: &str = "snapshot";

#[derive(Debug, Clone)]
pub struct History {
    root: PathBuf,
    keep: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub file: PathBuf,
    pub saved_at_ms: u128,
    pub len: u64,
}

impl History {
    /// `keep` is the number of snapshots retained per document.
    pub fn new(root: impl Into<PathBuf>, keep: usize) -> Self {
        Self {
            root: root.into(),
            keep: keep.max(1),
        }
    }

    fn dir_for(&self, document: &Path) -> PathBuf {
        let key = blake3::hash(document.as_os_str().as_encoded_bytes());
        self.root.join(&key.to_hex()[..24])
    }

    /// Store `bytes` as the newest snapshot of `document` and prune old ones.
    pub fn snapshot(&self, document: &Path, bytes: &[u8]) -> Result<PathBuf> {
        let dir = self.dir_for(document);
        fsio::create_private_dir_all(&dir)?;

        let origin = dir.join("origin");
        if !origin.exists() {
            fsio::atomic_write_private(&origin, document.as_os_str().as_encoded_bytes())?;
        }

        // Strictly increasing stamps: several saves within one millisecond
        // (or a clock that jumps backwards) must never produce a name that
        // sorts before an existing snapshot, or pruning would delete it.
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        let newest = list_dir(&dir)?.first().map_or(0, |s| s.saved_at_ms + 1);
        let stamp = now.max(newest);
        let file = dir.join(format!("{stamp:013}.{SNAPSHOT_EXT}"));
        fsio::atomic_write_private(&file, bytes)?;
        self.prune(&dir)?;
        Ok(file)
    }

    /// Snapshots of `document`, newest first.
    pub fn list(&self, document: &Path) -> Result<Vec<Snapshot>> {
        list_dir(&self.dir_for(document))
    }

    fn prune(&self, dir: &Path) -> Result<()> {
        for old in list_dir(dir)?.into_iter().skip(self.keep) {
            fs::remove_file(&old.file).map_err(|e| CoreError::io("remove", &old.file, e))?;
        }
        Ok(())
    }
}

fn list_dir(dir: &Path) -> Result<Vec<Snapshot>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(CoreError::io("list", dir, e)),
    };
    let mut out: Vec<Snapshot> = entries
        .filter_map(std::result::Result::ok)
        .filter_map(|entry| {
            let file = entry.path();
            if file.extension()? != SNAPSHOT_EXT {
                return None;
            }
            let saved_at_ms = file.file_stem()?.to_str()?.parse().ok()?;
            let len = entry.metadata().ok()?.len();
            Some(Snapshot {
                file,
                saved_at_ms,
                len,
            })
        })
        .collect();
    out.sort_by_key(|s| std::cmp::Reverse(s.saved_at_ms));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_the_newest_snapshots() {
        let root = tempfile::tempdir().unwrap();
        let history = History::new(root.path(), 3);
        let doc = Path::new("/tmp/some/doc.md");
        for i in 0..5u8 {
            history.snapshot(doc, &[i]).unwrap();
        }
        let snaps = history.list(doc).unwrap();
        assert_eq!(snaps.len(), 3);
        assert_eq!(fs::read(&snaps[0].file).unwrap(), [4]);
        assert_eq!(fs::read(&snaps[2].file).unwrap(), [2]);
    }

    #[test]
    fn documents_do_not_share_history() {
        let root = tempfile::tempdir().unwrap();
        let history = History::new(root.path(), 5);
        history.snapshot(Path::new("/a.md"), b"a").unwrap();
        assert!(history.list(Path::new("/b.md")).unwrap().is_empty());
    }
}
