//! Low-level, crash-safe filesystem primitives.

use std::fs::{self, File};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{CoreError, Result};

/// Text documents larger than this are refused instead of freezing the UI.
pub const MAX_TEXT_FILE_BYTES: u64 = 32 * 1024 * 1024;

/// Identity of a file's contents, used to detect external modifications.
/// Based on a content hash, not mtime, so it is immune to clock skew,
/// coarse timestamps and `touch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fingerprint {
    pub len: u64,
    pub hash: String,
}

impl Fingerprint {
    pub fn of(bytes: &[u8]) -> Self {
        Self {
            len: bytes.len() as u64,
            hash: blake3::hash(bytes).to_hex().to_string(),
        }
    }
}

/// Read a regular file, enforcing [`MAX_TEXT_FILE_BYTES`].
pub fn read_file(path: &Path) -> Result<Vec<u8>> {
    let meta = fs::metadata(path).map_err(|e| CoreError::io("read", path, e))?;
    if !meta.is_file() {
        return Err(CoreError::NotAFile {
            path: path.to_path_buf(),
        });
    }
    let too_large = |size| CoreError::TooLarge {
        path: path.to_path_buf(),
        size,
        limit: MAX_TEXT_FILE_BYTES,
    };
    if meta.len() > MAX_TEXT_FILE_BYTES {
        return Err(too_large(meta.len()));
    }
    let file = File::open(path).map_err(|e| CoreError::io("open", path, e))?;
    let mut buf = Vec::with_capacity(usize::try_from(meta.len()).unwrap_or(0));
    // `take` guards against a file that grows between `metadata` and `read`.
    file.take(MAX_TEXT_FILE_BYTES + 1)
        .read_to_end(&mut buf)
        .map_err(|e| CoreError::io("read", path, e))?;
    if buf.len() as u64 > MAX_TEXT_FILE_BYTES {
        return Err(too_large(buf.len() as u64));
    }
    Ok(buf)
}

/// Like [`read_file`] but a missing file is `Ok(None)`.
pub fn read_existing(path: &Path) -> Result<Option<Vec<u8>>> {
    match read_file(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(CoreError::Io { source, .. }) if source.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Resolve the real location a document should be written to.
///
/// Symlinks are followed so that saving through a link updates the target
/// instead of replacing the link with a regular file.
pub fn resolve_target(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return fs::canonicalize(path).map_err(|e| CoreError::io("resolve", path, e));
    }
    let name = path.file_name().ok_or_else(|| CoreError::NotAFile {
        path: path.to_path_buf(),
    })?;
    let parent = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    let parent = fs::canonicalize(parent).map_err(|e| CoreError::io("resolve", parent, e))?;
    Ok(parent.join(name))
}

/// Atomically replace `path` with `bytes` (new files get mode 0644 on Unix).
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    write_atomic(path, bytes, 0o644)
}

/// Same as [`atomic_write`] but new files are private to the user (0600).
pub fn atomic_write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    write_atomic(path, bytes, 0o600)
}

/// temp file in the same directory → write → fsync → rename → fsync dir.
///
/// A crash at any point leaves either the old or the new file, never a
/// truncated one. Existing permissions are preserved. Note: like every
/// rename-based save, this replaces the inode (hard links are not followed).
fn write_atomic(path: &Path, bytes: &[u8], new_file_mode: u32) -> Result<()> {
    let dir = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    let name = path.file_name().ok_or_else(|| CoreError::NotAFile {
        path: path.to_path_buf(),
    })?;

    let mut tmp = tempfile::Builder::new()
        .prefix(&format!(".{}.", name.to_string_lossy()))
        .suffix(".fa-tmp")
        .tempfile_in(dir)
        .map_err(|e| CoreError::io("create a temporary file in", dir, e))?;

    tmp.write_all(bytes)
        .map_err(|e| CoreError::io("write", tmp.path(), e))?;

    let set_permissions = match fs::metadata(path) {
        Ok(meta) => tmp.as_file().set_permissions(meta.permissions()),
        Err(_) => set_new_file_mode(tmp.as_file(), new_file_mode),
    };
    set_permissions.map_err(|e| CoreError::io("set permissions on", tmp.path(), e))?;

    tmp.as_file()
        .sync_all()
        .map_err(|e| CoreError::io("flush", tmp.path(), e))?;
    tmp.persist(path)
        .map_err(|e| CoreError::io("replace", path, e.error))?;
    sync_dir(dir)
}

#[cfg(unix)]
fn set_new_file_mode(file: &File, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    file.set_permissions(fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
fn set_new_file_mode(_file: &File, _mode: u32) -> std::io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn sync_dir(dir: &Path) -> Result<()> {
    File::open(dir)
        .and_then(|d| d.sync_all())
        .map_err(|e| CoreError::io("flush directory", dir, e))
}

#[cfg(not(unix))]
#[allow(clippy::unnecessary_wraps)]
fn sync_dir(_dir: &Path) -> Result<()> {
    Ok(())
}

/// Create a directory tree readable only by the current user.
pub fn create_private_dir_all(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir).map_err(|e| CoreError::io("create directory", dir, e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
            .map_err(|e| CoreError::io("set permissions on", dir, e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_replaces_and_leaves_no_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("doc.md");
        atomic_write(&path, b"one").unwrap();
        atomic_write(&path, b"two").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"two");
        let entries: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(entries.len(), 1, "temp file left behind");
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_preserves_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("script.md");
        fs::write(&path, b"x").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        atomic_write(&path, b"y").unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640);
    }

    #[cfg(unix)]
    #[test]
    fn resolve_target_follows_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real.md");
        let link = dir.path().join("link.md");
        fs::write(&real, b"x").unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let target = resolve_target(&link).unwrap();
        atomic_write(&target, b"y").unwrap();
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read(&real).unwrap(), b"y");
    }

    #[test]
    fn directories_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            read_file(dir.path()),
            Err(CoreError::NotAFile { .. })
        ));
    }
}
