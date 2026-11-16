# ADR 0001 — Application stack

- Status: accepted
- Date: 2026-11-16

## Context

We want a cross-platform desktop editor for standard text formats, Markdown
first, with a refined minimalist UX, strong data safety, and a path to open
source distribution (Flathub, Microsoft Store, Mac App Store, …).

## Decision

1. **Tauri 2 (stable) as the shell**, not a pure-Rust GPU UI (GPUI, iced,
   egui, Slint). Rich text editing — IME, accessibility, typography, embedded
   code highlighting, math and diagrams later — is years ahead on the web
   platform. Tauri has official distribution guides for every target store.
   Risk: WebKitGTK on Linux (performance, NVIDIA quirks). CEF is only in the
   Tauri 3 alpha and adds hundreds of MB; revisit when it is stable.

2. **CodeMirror 6 with live-preview decorations, not a ProseMirror-family
   WYSIWYG** (Milkdown, Tiptap). Tree-based editors parse Markdown into a schema
   and re-serialise it on save, which reformats untouched content (escapes,
   list markers, blank lines). CodeMirror keeps the source as the single
   source of truth; rendering is a view over it. Cost: we own the
   decoration layer and widgets (tables, images).

3. **A UI-independent Rust core (`fa-core`)** owns every byte that reaches
   disk: lossless decoding, atomic writes, conflict detection by content hash,
   version history. It can back a different shell (native, CLI) later.

4. **Svelte 5** for the chrome around the editor: small runtime, fine-grained
   reactivity, stays out of CodeMirror's way.

## Security model

- Webview gets no `fs`, `shell` or `dialog` plugin permissions. All app
  commands are declared in `build.rs` and granted explicitly per window in
  `capabilities/main.json` (fails closed).
- The webview cannot name arbitrary paths: Rust only opens paths that arrived
  through a user gesture (file dialog, CLI, drag & drop, recent list).
- Strict CSP (`default-src 'self'`, no remote scripts, no frames, no network).
- External links: http(s)/mailto only, confirmed by the user, opened by Rust.
- Remote images will be blocked by default (tracking pixels).

## Consequences

- Live preview must be built and maintained in-house (`livePreview.ts`).
- Linux rendering quality depends on the distro's WebKitGTK.
