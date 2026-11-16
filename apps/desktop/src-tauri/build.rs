// Every app command is listed here so that Tauri generates a permission for
// it; a command is only callable from a window whose capability grants it
// (see capabilities/main.json). Forgetting to grant one fails closed.
const COMMANDS: &[&str] = &[
    "app_take_startup_paths",
    "doc_new",
    "doc_open_dialog",
    "doc_open_path",
    "doc_save",
    "doc_save_as",
    "doc_reload",
    "doc_close",
    "doc_disk_state",
    "recent_list",
    "open_external",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to run tauri-build");
}
