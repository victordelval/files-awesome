# Files Awesome

A calm, safe desktop editor for plain-text formats — starting with Markdown.

- **Live preview that never rewrites your file.** Markdown is rendered in place,
  but the document stays the exact text you typed. Opening and saving an
  untouched file is byte-identical (BOM and CRLF included).
- **Crash-safe saves.** Temp file → fsync → atomic rename → fsync directory.
- **No silent overwrites.** If another program changes the file, you are told
  before anything is replaced. Previous versions go to a private local history.
- **Locked-down by default.** The webview has no filesystem, shell or network
  access; it can only ask the Rust core to open files *you* picked.

> Status: early MVP (v0.1). Linux is the primary dev platform; macOS and
> Windows builds are expected to work but are not yet tested.

## Stack

| Layer | Choice |
|---|---|
| Shell | [Tauri 2](https://v2.tauri.app) (system WebView) |
| Core | Rust crate `fa-core` — UI-independent document I/O |
| UI | Svelte 5 + TypeScript + Vite |
| Editor | CodeMirror 6, live preview as decorations over the source |

Rationale: [docs/adr/0001-stack.md](docs/adr/0001-stack.md).

## Layout

```
crates/fa-core/          Rust core: text codec, atomic I/O, conflicts, history, recents
apps/desktop/src-tauri/  Tauri shell: IPC commands, file watcher, permissions (ACL)
apps/desktop/src/        UI: controller, CodeMirror setup, live preview, components
```

## Develop

Requirements: Rust ≥ 1.88, Node ≥ 20, pnpm, and the
[Tauri system dependencies](https://v2.tauri.app/start/prerequisites/)
(on Debian/Ubuntu: `libwebkit2gtk-4.1-dev build-essential libssl-dev librsvg2-dev`).

```sh
pnpm install            # once
pnpm dev                # native app with hot reload
pnpm dev:web            # UI only, in a browser, with an in-memory demo backend
pnpm check              # svelte-check + clippy + tests
pnpm build              # release bundles (deb, rpm, AppImage / msi / dmg)
```

Open a file directly: `target/debug/files-awesome notes.md`.

## Keyboard

| | |
|---|---|
| `Ctrl+P` | Command palette |
| `Ctrl+N` / `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S` | New / Open / Save / Save as |
| `Ctrl+/` | Toggle live preview ↔ source |
| `Ctrl+.` | Focus mode (`Esc` to leave) |
| `Ctrl+Shift+O` | Toggle outline |
| `Ctrl+F` | Find & replace |
| `Ctrl+B` `Ctrl+I` `Ctrl+E` `Ctrl+K` | Bold, italic, code, link |
| `Ctrl+1…6` | Heading level (same level again → paragraph) |
| `Ctrl+=` / `Ctrl+-` / `Ctrl+0` | Text size |

`Cmd` replaces `Ctrl` on macOS.

## Contributing

Early days: the architecture and UX are still moving. Issues with ideas, bugs
or Markdown files that render wrong are very welcome. Please open an issue
before a large pull request.

Security issues: see [SECURITY.md](SECURITY.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

Bundled fonts (Geist, Geist Mono, Newsreader) are distributed under the SIL
Open Font License 1.1.
