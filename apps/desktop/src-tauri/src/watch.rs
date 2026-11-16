//! Notifies the UI when an open document changes on disk.
//!
//! We watch the *parent directory* (non-recursively) rather than the file:
//! many programs save by writing a new file and renaming it over the old one,
//! which silently ends a watch on the original inode.
//!
//! Events are only hints. The UI re-checks with `doc_disk_state`, which
//! compares content hashes, so our own saves never show up as conflicts.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use notify_debouncer_full::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};
use tauri::{AppHandle, Emitter};

pub const DISK_CHANGED_EVENT: &str = "doc:disk-changed";

type Targets = Arc<Mutex<HashMap<PathBuf, u32>>>;

pub struct DocWatcher {
    debouncer: Mutex<Option<Debouncer<RecommendedWatcher, RecommendedCache>>>,
    targets: Targets,
    dirs: Mutex<HashMap<PathBuf, usize>>,
}

impl DocWatcher {
    pub fn new(app: AppHandle) -> Self {
        let targets: Targets = Arc::default();
        let shared = Arc::clone(&targets);
        let debouncer = new_debouncer(
            Duration::from_millis(250),
            None,
            move |result: DebounceEventResult| {
                let Ok(events) = result else { return };
                let hits: BTreeSet<u32> = {
                    let targets = shared.lock().unwrap_or_else(PoisonError::into_inner);
                    events
                        .iter()
                        .flat_map(|event| event.paths.iter())
                        .filter_map(|path| targets.get(path).copied())
                        .collect()
                };
                for id in hits {
                    let _ = app.emit(DISK_CHANGED_EVENT, id);
                }
            },
        );
        if let Err(e) = &debouncer {
            eprintln!("file watching disabled: {e}");
        }
        Self {
            debouncer: Mutex::new(debouncer.ok()),
            targets,
            dirs: Mutex::default(),
        }
    }

    pub fn watch(&self, id: u32, file: &Path) {
        self.unwatch(id);
        let Some(dir) = file.parent() else { return };
        self.targets
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(file.to_path_buf(), id);

        let mut dirs = self.dirs.lock().unwrap_or_else(PoisonError::into_inner);
        let count = dirs.entry(dir.to_path_buf()).or_insert(0);
        *count += 1;
        if *count == 1
            && let Some(debouncer) = self
                .debouncer
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .as_mut()
            && let Err(e) = debouncer.watch(dir, RecursiveMode::NonRecursive)
        {
            eprintln!("cannot watch {}: {e}", dir.display());
        }
    }

    pub fn unwatch(&self, id: u32) {
        let removed: Vec<PathBuf> = {
            let mut targets = self.targets.lock().unwrap_or_else(PoisonError::into_inner);
            let files: Vec<PathBuf> = targets
                .iter()
                .filter(|(_, v)| **v == id)
                .map(|(k, _)| k.clone())
                .collect();
            for file in &files {
                targets.remove(file);
            }
            files
        };

        let mut dirs = self.dirs.lock().unwrap_or_else(PoisonError::into_inner);
        for dir in removed.iter().filter_map(|f| f.parent()) {
            let Some(count) = dirs.get_mut(dir) else {
                continue;
            };
            *count -= 1;
            if *count == 0 {
                dirs.remove(dir);
                if let Some(debouncer) = self
                    .debouncer
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .as_mut()
                {
                    let _ = debouncer.unwatch(dir);
                }
            }
        }
    }
}
