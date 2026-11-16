//! Most-recently-used document list, persisted as JSON.

use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::fsio;

#[derive(Debug, Clone)]
pub struct RecentFiles {
    items: Vec<PathBuf>,
    max: usize,
}

impl RecentFiles {
    pub fn new(max: usize) -> Self {
        Self {
            items: Vec::new(),
            max,
        }
    }

    /// Load from `file`; a missing or corrupt list simply starts empty.
    pub fn load(file: &Path, max: usize) -> Self {
        let items = std::fs::read(file)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Vec<PathBuf>>(&bytes).ok())
            .unwrap_or_default();
        let mut list = Self { items, max };
        list.items.truncate(max);
        list
    }

    pub fn save(&self, file: &Path) -> Result<()> {
        if let Some(dir) = file.parent() {
            fsio::create_private_dir_all(dir)?;
        }
        let json = serde_json::to_vec_pretty(&self.items).unwrap_or_default();
        fsio::atomic_write_private(file, &json)
    }

    /// Move `path` to the front.
    pub fn touch(&mut self, path: &Path) {
        self.items.retain(|p| p != path);
        self.items.insert(0, path.to_path_buf());
        self.items.truncate(self.max);
    }

    pub fn remove(&mut self, path: &Path) {
        self.items.retain(|p| p != path);
    }

    pub fn contains(&self, path: &Path) -> bool {
        self.items.iter().any(|p| p == path)
    }

    pub fn items(&self) -> &[PathBuf] {
        &self.items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_dedupes_and_caps() {
        let mut list = RecentFiles::new(2);
        list.touch(Path::new("/a"));
        list.touch(Path::new("/b"));
        list.touch(Path::new("/a"));
        list.touch(Path::new("/c"));
        assert_eq!(list.items(), [PathBuf::from("/c"), PathBuf::from("/a")]);
    }

    #[test]
    fn persists_and_tolerates_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("cfg").join("recent.json");
        let mut list = RecentFiles::new(5);
        list.touch(Path::new("/x.md"));
        list.save(&file).unwrap();
        assert_eq!(RecentFiles::load(&file, 5).items(), list.items());
        std::fs::write(&file, b"{not json").unwrap();
        assert_eq!(RecentFiles::load(&file, 5).items(), [] as [PathBuf; 0]);
    }
}
