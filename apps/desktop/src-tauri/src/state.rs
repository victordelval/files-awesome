use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use fa_core::{Document, History, RecentFiles};
use tauri::{AppHandle, Manager};

use crate::watch::DocWatcher;

const RECENT_MAX: usize = 12;
const HISTORY_KEEP: usize = 50;

pub struct AppState {
    inner: Mutex<Inner>,
    pub watcher: DocWatcher,
    pub history: Option<History>,
    recent_file: Option<PathBuf>,
}

pub struct Inner {
    next_id: u32,
    pub docs: HashMap<u32, Document>,
    pub recent: RecentFiles,
    /// Paths the user handed to us through a trusted channel (file dialog,
    /// command line, drag & drop, recent list). The webview can only ask to
    /// open paths from this set — it can never name an arbitrary file.
    granted: HashSet<PathBuf>,
    startup: Vec<PathBuf>,
}

impl AppState {
    pub fn new(app: &AppHandle, startup: Vec<PathBuf>) -> Self {
        let paths = app.path();
        let recent_file = paths.app_config_dir().ok().map(|d| d.join("recent.json"));
        let history = paths
            .app_data_dir()
            .ok()
            .map(|d| History::new(d.join("history"), HISTORY_KEEP));

        let recent = recent_file.as_deref().map_or_else(
            || RecentFiles::new(RECENT_MAX),
            |f| RecentFiles::load(f, RECENT_MAX),
        );
        let mut granted: HashSet<PathBuf> = recent.items().iter().cloned().collect();
        granted.extend(startup.iter().cloned());

        Self {
            inner: Mutex::new(Inner {
                next_id: 1,
                docs: HashMap::new(),
                recent,
                granted,
                startup,
            }),
            watcher: DocWatcher::new(app.clone()),
            history,
            recent_file,
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn persist_recent(&self, inner: &Inner) {
        if let Some(file) = &self.recent_file
            && let Err(e) = inner.recent.save(file)
        {
            eprintln!("could not save recent files: {e}");
        }
    }
}

impl Inner {
    pub fn insert(&mut self, doc: Document) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.docs.insert(id, doc);
        id
    }

    pub fn grant(&mut self, path: &Path) {
        self.granted.insert(path.to_path_buf());
        if let Ok(canonical) = std::fs::canonicalize(path) {
            self.granted.insert(canonical);
        }
    }

    pub fn is_granted(&self, path: &Path) -> bool {
        self.granted.contains(path)
            || std::fs::canonicalize(path).is_ok_and(|c| self.granted.contains(&c))
    }

    pub fn take_startup(&mut self) -> Vec<PathBuf> {
        std::mem::take(&mut self.startup)
    }
}

/// File paths from a command line (`argv[0]` excluded), resolved against `cwd`.
pub fn paths_from_args(args: impl IntoIterator<Item = String>, cwd: &Path) -> Vec<PathBuf> {
    args.into_iter()
        .filter(|a| !a.starts_with('-'))
        .map(|a| {
            let p = PathBuf::from(a);
            if p.is_absolute() { p } else { cwd.join(p) }
        })
        .filter(|p| p.is_file())
        .collect()
}
