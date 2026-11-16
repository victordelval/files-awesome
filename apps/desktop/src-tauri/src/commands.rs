//! IPC surface exposed to the webview.
//!
//! Security model: the front end never passes a filesystem path that we have
//! not seen come from the user (dialog, CLI, drop, recent list). Documents are
//! addressed by opaque numeric ids afterwards.

use std::path::{Path, PathBuf};

use fa_core::{DiskState, Document, SaveMode, TextFormat};
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::error::{CmdError, CmdResult};
use crate::state::AppState;

const MARKDOWN_EXTENSIONS: &[&str] = &["md", "markdown", "mdown", "mkd", "mdx"];
const TEXT_EXTENSIONS: &[&str] = &["txt", "text"];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocMeta {
    id: u32,
    path: Option<String>,
    name: String,
    dir: Option<String>,
    format: TextFormat,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocPayload {
    #[serde(flatten)]
    meta: DocMeta,
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePayload {
    meta: DocMeta,
    wrote: bool,
    history_warning: Option<String>,
}

#[derive(Serialize)]
pub struct RecentEntry {
    path: String,
    name: String,
    dir: String,
}

fn meta(id: u32, doc: &Document) -> DocMeta {
    let path = doc.path();
    DocMeta {
        id,
        path: path.map(|p| p.display().to_string()),
        name: path
            .and_then(Path::file_name)
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
        dir: path.and_then(Path::parent).map(|p| p.display().to_string()),
        format: doc.format(),
    }
}

fn open_granted(state: &AppState, path: &Path) -> CmdResult<DocPayload> {
    let (doc, text) = Document::open(path)?;
    let canonical = doc.path().map(Path::to_path_buf);
    let mut inner = state.lock();
    let id = inner.insert(doc);
    if let Some(p) = &canonical {
        inner.grant(p);
        inner.recent.touch(p);
        state.persist_recent(&inner);
        state.watcher.watch(id, p);
    }
    Ok(DocPayload {
        meta: meta(id, &inner.docs[&id]),
        text,
    })
}

#[tauri::command]
pub async fn app_take_startup_paths(state: State<'_, AppState>) -> CmdResult<Vec<String>> {
    Ok(state
        .lock()
        .take_startup()
        .iter()
        .map(|p| p.display().to_string())
        .collect())
}

#[tauri::command]
pub async fn doc_new(state: State<'_, AppState>) -> CmdResult<DocPayload> {
    let mut inner = state.lock();
    let id = inner.insert(Document::untitled());
    Ok(DocPayload {
        meta: meta(id, &inner.docs[&id]),
        text: String::new(),
    })
}

#[tauri::command]
pub async fn doc_open_dialog(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<Option<DocPayload>> {
    let picked = app
        .dialog()
        .file()
        .add_filter("Markdown", MARKDOWN_EXTENSIONS)
        .add_filter("Texto", TEXT_EXTENSIONS)
        .blocking_pick_file();
    let Some(picked) = picked else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(CmdError::internal)?;
    state.lock().grant(&path);
    open_granted(&state, &path).map(Some)
}

/// Open a path previously granted by the user (recent list, CLI, drop).
#[tauri::command]
pub async fn doc_open_path(path: String, state: State<'_, AppState>) -> CmdResult<DocPayload> {
    let path = PathBuf::from(path);
    if !state.lock().is_granted(&path) {
        return Err(CmdError::new(
            "forbidden",
            "this path was not opened by the user",
        ));
    }
    let result = open_granted(&state, &path);
    if result.is_err() && !path.exists() {
        let mut inner = state.lock();
        inner.recent.remove(&path);
        state.persist_recent(&inner);
    }
    result
}

#[tauri::command]
pub async fn doc_save(
    id: u32,
    text: String,
    overwrite: bool,
    state: State<'_, AppState>,
) -> CmdResult<SavePayload> {
    let mut inner = state.lock();
    let doc = inner.docs.get_mut(&id).ok_or(CmdError::unknown_doc(id))?;
    let mode = if overwrite {
        SaveMode::Overwrite
    } else {
        SaveMode::Safe
    };
    let outcome = doc.save(&text, mode, state.history.as_ref())?;
    Ok(SavePayload {
        meta: meta(id, doc),
        wrote: outcome.wrote,
        history_warning: outcome.history_warning,
    })
}

#[tauri::command]
pub async fn doc_save_as(
    id: u32,
    text: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<Option<SavePayload>> {
    let (suggested, start_dir) = {
        let inner = state.lock();
        let doc = inner.docs.get(&id).ok_or(CmdError::unknown_doc(id))?;
        let name = doc.path().and_then(Path::file_name).map_or_else(
            || "Sin título.md".to_owned(),
            |n| n.to_string_lossy().into_owned(),
        );
        (
            name,
            doc.path().and_then(Path::parent).map(Path::to_path_buf),
        )
    };

    let mut dialog = app
        .dialog()
        .file()
        .set_file_name(suggested)
        .add_filter("Markdown", MARKDOWN_EXTENSIONS);
    if let Some(dir) = start_dir {
        dialog = dialog.set_directory(dir);
    }
    let Some(picked) = dialog.blocking_save_file() else {
        return Ok(None);
    };
    let mut path = picked.into_path().map_err(CmdError::internal)?;
    if path.extension().is_none() {
        path.set_extension("md");
    }

    let mut inner = state.lock();
    let doc = inner.docs.get_mut(&id).ok_or(CmdError::unknown_doc(id))?;
    let outcome = doc.save_as(&path, &text, state.history.as_ref())?;
    let saved_meta = meta(id, doc);
    if let Some(p) = doc.path().map(Path::to_path_buf) {
        state.watcher.watch(id, &p);
        inner.grant(&p);
        inner.recent.touch(&p);
        state.persist_recent(&inner);
    }
    Ok(Some(SavePayload {
        meta: saved_meta,
        wrote: outcome.wrote,
        history_warning: outcome.history_warning,
    }))
}

#[tauri::command]
pub async fn doc_reload(id: u32, state: State<'_, AppState>) -> CmdResult<DocPayload> {
    let mut inner = state.lock();
    let doc = inner.docs.get_mut(&id).ok_or(CmdError::unknown_doc(id))?;
    let text = doc.reload()?;
    Ok(DocPayload {
        meta: meta(id, doc),
        text,
    })
}

#[tauri::command]
pub async fn doc_close(id: u32, state: State<'_, AppState>) -> CmdResult<()> {
    state.lock().docs.remove(&id);
    state.watcher.unwatch(id);
    Ok(())
}

#[tauri::command]
pub async fn doc_disk_state(id: u32, state: State<'_, AppState>) -> CmdResult<DiskState> {
    let inner = state.lock();
    let doc = inner.docs.get(&id).ok_or(CmdError::unknown_doc(id))?;
    Ok(doc.disk_state()?)
}

#[tauri::command]
pub async fn recent_list(state: State<'_, AppState>) -> CmdResult<Vec<RecentEntry>> {
    let inner = state.lock();
    Ok(inner
        .recent
        .items()
        .iter()
        .filter(|p| p.is_file())
        .map(|p| RecentEntry {
            path: p.display().to_string(),
            name: p
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
            dir: p
                .parent()
                .map_or_else(String::new, |d| d.display().to_string()),
        })
        .collect())
}

/// Open a web or mail link in the system browser. Only `http`, `https` and
/// `mailto` are accepted; the UI asks the user before calling this.
#[tauri::command]
pub async fn open_external(url: String, app: AppHandle) -> CmdResult<()> {
    let lower = url.to_ascii_lowercase();
    let allowed = ["https://", "http://", "mailto:"]
        .iter()
        .any(|scheme| lower.starts_with(scheme));
    if !allowed || url.len() > 4096 || url.chars().any(char::is_control) {
        return Err(CmdError::new(
            "forbidden",
            "only http(s) and mailto links can be opened",
        ));
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(CmdError::internal)
}
