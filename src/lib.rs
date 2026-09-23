//! `FunnyFileManager` (FFM) — a terminal dual-panel file manager inspired by
//! Far Manager, built on the `ratatui` TUI framework.
//!
//! # Architecture
//!
//! FFM runs a classic TUI loop around a central [`App`](models::App). An active
//! [`Overlay`](models::Overlay) receives every key first; otherwise the key is
//! resolved into a typed [`Action`](action::Action), applied, and re-rendered.
//! Responsibilities are split across focused modules:
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `application` | Event loop, modal-first input routing, and action dispatch |
//! | `diagnostics` | Opt-in, payload-free logging and run outcome reporting |
//! | [`models`]   | Core state: [`App`](models::App), [`Panel`](models::Panel), `DirEntry`, and the `Overlay` enum (single source of modal state) |
//! | [`ui`]       | Pure rendering of panels, status bar, command bar, and overlays |
//! | [`action`]   | Declarative bindings for typed main-view and editor commands |
//! | [`command`]  | Typed command-line DSL (`Command::parse`) for the `:` prompt |
//! | [`ops`]      | Canonical command, file, shell, viewer, and MCP operations |
//! | [`popup`]    | Modal input handling and overlay state transitions |
//! | [`handlers`] | Compatibility re-exports for the former operation API |
//! | [`utils`]    | File utilities (`format_size`, `copy_dir_all`, `unique_destination`, `refresh_panels`) |
//! | [`editor`]   | Built-in F4 editor and external Alt+F4 integration |
//! | `terminal`   | RAII lifecycle for the application terminal session and external-editor suspension |
//! | [`s3`]       | Optional AWS S3 backend (enabled with `--features s3`) |
//! | [`windows`]  | Windows-specific drive and diagnostic helpers |
//!
//! # Data flow
//!
//! ```text
//! key event ─▶ active Overlay? ─yes─▶ popup (key consumed) ─┐
//!                  │                                        │
//!                  no                                       ▼
//!                  └─▶ action::resolve ─▶ application::dispatch ─▶ App ─▶ ui::ui()
//! ```
//!
//! # Key design decisions
//!
//! - **One overlay at a time:** all modal state lives in a single `Overlay` enum,
//!   so two dialogs can never be open simultaneously — illegal states are
//!   unrepresentable at compile time. While one is active, no key falls through
//!   to the main-view action resolver.
//! - **One operation owner:** implementations live under `ops`; `handlers` is
//!   only a backward-compatible facade, and `popup` depends directly on `ops`.
//! - **Terminal cleanup by construction:** RAII guards own best-effort terminal
//!   restoration after normal return, setup failure, or unwinding, including
//!   around an external editor.
//! - **Feature-invariant runtime:** `test-helpers` exposes explicit test seams;
//!   it never selects alternate `run`, `main_impl`, or external-editor behavior.
//! - **Dual panels:** independent left/right [`Panel`](models::Panel)s; `Tab`
//!   switches the active one, `Ctrl+U` swaps their contents.
//! - **Rename-with-fallback:** moves try an atomic `rename`, then fall back to
//!   copy + delete when crossing filesystems.
//! - **Never overwrite:** destinations get a `_N` suffix via `unique_destination`.

// Unit-test ergonomics only; the non-test library build still enforces all four.
#![cfg_attr(
    test,
    allow(
        clippy::unnecessary_wraps,
        clippy::needless_pass_by_value,
        clippy::needless_pass_by_ref_mut,
        clippy::match_wildcard_for_single_variants
    )
)]

pub mod action;
mod application;
mod bootstrap;
mod diagnostics;
mod directory;
mod recording;
pub use application::{run_app, run_app_with_event_reader};
pub use bootstrap::run;

pub mod associations;

#[cfg(feature = "keepass")]
pub use application::CLIPBOARD_CLEAR;
pub mod casefold;
pub mod clipboard;
pub mod clock;
pub mod command;
pub mod complete;
pub mod config;
pub mod console;
pub mod copy;
pub mod cycle;
pub mod description;
pub mod diskfree;
pub mod editor;
pub mod encoding;
mod fileload;
pub mod git;
pub mod handlers;
pub mod highlight;
pub mod history;
pub mod i18n;
#[cfg(feature = "keepass")]
pub mod kdbx;
#[cfg(feature = "keepass")]
pub mod keepass;
pub mod lineedit;
pub(crate) mod lsp;
pub mod mask;
pub mod menu;
pub mod models;
pub mod mouse;
pub mod ops;
pub mod packages;
mod panel_session;
pub(crate) mod path_display;
pub mod path_env;
pub mod performance;
mod platform;
pub mod popup;
pub mod prompt;
pub mod quicksearch;
#[cfg(feature = "s3")]
pub mod s3;
pub mod scroll;
pub mod search;
pub mod stats;
pub mod status;
mod terminal;
pub mod ui;
pub mod usermenu;
pub mod utils;
pub mod windows;
pub mod wsl;

#[cfg(any(test, feature = "test-helpers"))]
pub use application::run_app_with_wsl_backend;

#[cfg(any(test, coverage, feature = "test-helpers"))]
pub mod test_support;
#[cfg(any(test, coverage, feature = "test-helpers"))]
pub use test_support::WritableTestBackend;

#[cfg(coverage)]
pub use test_support::{coverage_touch_backend, coverage_touch_logs};
