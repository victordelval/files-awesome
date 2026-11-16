mod commands;
mod error;
mod state;
mod watch;

use std::path::PathBuf;

use tauri::{DragDropEvent, Emitter, Manager, WindowEvent};

use state::{AppState, paths_from_args};

/// Emitted with `string[]` when the user hands us files while running
/// (second launch with arguments, drag & drop onto the window).
const OPEN_PATHS_EVENT: &str = "app:open-paths";

fn grant_and_emit(app: &tauri::AppHandle, paths: Vec<PathBuf>) {
    if paths.is_empty() {
        return;
    }
    let state = app.state::<AppState>();
    {
        let mut inner = state.lock();
        for p in &paths {
            inner.grant(p);
        }
    }
    let list: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
    let _ = app.emit(OPEN_PATHS_EVENT, list);
}

/// Start the desktop application.
///
/// # Panics
/// If the Tauri runtime cannot start (e.g. no display or missing `WebView`).
pub fn run() {
    tauri::Builder::default()
        // Must be registered first: a second launch forwards its files here.
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
            let paths = paths_from_args(argv.into_iter().skip(1), &PathBuf::from(cwd));
            grant_and_emit(app, paths);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let cwd = std::env::current_dir().unwrap_or_default();
            let startup = paths_from_args(std::env::args().skip(1), &cwd);
            app.manage(AppState::new(app.handle(), startup));

            // The UI shows the window once it has painted. If it never does
            // (broken frontend), don't leave an invisible process behind.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(4));
                if let Some(window) = handle.get_webview_window("main")
                    && !window.is_visible().unwrap_or(true)
                {
                    let _ = window.show();
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. }) = event {
                let files = paths.iter().filter(|p| p.is_file()).cloned().collect();
                grant_and_emit(window.app_handle(), files);
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_take_startup_paths,
            commands::doc_new,
            commands::doc_open_dialog,
            commands::doc_open_path,
            commands::doc_save,
            commands::doc_save_as,
            commands::doc_reload,
            commands::doc_close,
            commands::doc_disk_state,
            commands::recent_list,
            commands::open_external,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Files Awesome");
}
