//! `fa-core` — the UI-independent heart of Files Awesome.
//!
//! Everything that touches the user's files lives here, so it can be tested
//! without a window and reused by any front end (Tauri today, maybe a native
//! GPUI shell or a CLI tomorrow).
//!
//! Guarantees this crate is built around:
//!
//! * **Byte fidelity** — opening and saving an unmodified document produces the
//!   exact same bytes (BOM and line endings are detected and restored).
//! * **Crash safety** — saves are atomic (temp file → fsync → rename → fsync dir).
//! * **No silent overwrites** — a save refuses to clobber a file that another
//!   program changed since it was opened, unless explicitly told to.
//! * **Undo for the disk** — the previous on-disk version is snapshotted to a
//!   private history folder before every overwrite.
#![forbid(unsafe_code)]

pub mod document;
pub mod error;
pub mod fsio;
pub mod history;
pub mod recent;
pub mod text;

pub use document::{DiskState, Document, SaveMode, SaveOutcome};
pub use error::{CoreError, Result};
pub use fsio::Fingerprint;
pub use history::History;
pub use recent::RecentFiles;
pub use text::{LineEnding, TextFormat};
