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
//! | [`models`]   | Core state: [`App`](models::App), [`Panel`](models::Panel), `DirEntry`, and the `Overlay` enum (single source of modal state) |
//! | [`ui`]       | Pure rendering of panels, status bar, command bar, and overlays |
//! | [`action`]   | Pure main-view key mapping (`KeyEvent` → [`Action`](action::Action)) |
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
//!                  └─▶ action::resolve ─▶ apply_action ─▶ App ─▶ ui::ui()
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

pub mod action;
mod bootstrap;
pub use bootstrap::run;
pub mod command;
pub mod complete;
pub mod console;
pub mod editor;
mod fileload;
pub mod handlers;
pub mod history;
#[cfg(feature = "keepass")]
pub mod keepass;
pub mod lineedit;
pub mod models;
pub mod mutation_test;
pub mod ops;
pub mod popup;
pub mod quicksearch;
#[cfg(feature = "s3")]
pub mod s3;
pub mod scroll;
pub mod status;
mod terminal;
pub mod ui;
pub mod utils;
pub mod windows;

#[cfg(test)]
mod editor_unit_tests;

#[cfg(any(test, feature = "test-helpers"))]
pub mod test_support;

use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{backend::Backend, Terminal};
use std::{error::Error, io, io::Write};

use crate::action::{resolve, Action};
use crate::editor::{handle_edit, handle_edit_builtin};
use crate::models::{App, CommandLine, Lifecycle, Overlay, ViewMode};
use crate::ops::{handle_copy, handle_delete_confirm, handle_mkdir, handle_move, handle_view};
use crate::popup::process_popup_input;
use crate::ui::ui;
use std::time::{Duration, Instant};

/// Directory the logs are written to, or `None` when logging is off.
///
/// Logging is opt-in (`FFM_LOG=1`) and off by default: the event loop records
/// every key press *before* the overlays consume it, so an always-on log turns
/// whatever is typed into the editor, the `:` prompt or the filename dialog —
/// passwords, tokens — into a file on disk. It also lands in a per-user temp
/// directory rather than the working directory, so it neither litters the
/// folder `ffm` was started from nor depends on it. Read once: the environment
/// cannot change mid-run.
fn log_dir() -> Option<&'static Path> {
    static DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
    DIR.get_or_init(|| {
        std::env::var("FFM_LOG")
            .is_ok_and(|value| value == "1")
            .then(std::env::temp_dir)
    })
    .as_deref()
}

/// Append one timestamped line to `path`, creating it owner-only where the OS
/// supports it. Failures are ignored on purpose: logging must never be able to
/// break the file manager.
fn append_log_line(path: &Path, line: &str) {
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        // 0600 — the log records the paths of whoever ran ffm; the default umask
        // would leave it readable by every other local user.
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let Ok(mut file) = options.open(path) else {
        return;
    };
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let _ = writeln!(file, "[{timestamp}] {line}");
}

fn log_action_to(path: &Path, action: &str) {
    append_log_line(path, action);
}

fn log_error_to(path: &Path, error: &str) {
    append_log_line(path, &format!("ERROR: {error}"));
}

pub(crate) fn log_action(action: &str) {
    if let Some(dir) = log_dir() {
        log_action_to(&dir.join("ffm_actions.log"), action);
    }
}

#[allow(dead_code)]
fn log_error(error: &str) {
    if let Some(dir) = log_dir() {
        log_error_to(&dir.join("ffm_errors.log"), error);
    }
}

/// Describe an action for the log *without* any text the user typed.
///
/// Both `QuickSearch` and `CommandLineChar` carry a character straight off the
/// keyboard, and actions are logged before an overlay consumes them — so a
/// derived `Debug` would write every password typed at the `:` prompt into the
/// log file, one keystroke per line. Same rule, and same reason, as
/// [`key_label`]; keeping it a pure function is what lets a test prove it.
fn action_label(action: Action) -> String {
    match action {
        Action::QuickSearch(_) => "QuickSearch(..)".to_owned(),
        Action::CommandLineChar(_) => "CommandLineChar(..)".to_owned(),
        other => format!("{other:?}"),
    }
}

/// Describe a key for the log *without* its payload.
///
/// `KeyCode::Char` is rendered payload-free on purpose: keys are logged before
/// an overlay gets to consume them, so printing the character would write every
/// password typed into the built-in editor straight to the log file.
fn key_label(code: KeyCode) -> String {
    match code {
        KeyCode::Char(_) => "Char".to_owned(),
        other => format!("{other:?}"),
    }
}

#[cfg(coverage)]
pub fn coverage_touch_logs() {
    // Exercise non-test instantiations in coverage builds.
    log_action("coverage log action");
    log_error("coverage log error");
}

#[cfg(coverage)]
pub fn coverage_touch_backend() -> io::Result<()> {
    // Drive WritableTestBackend trait methods in non-test builds.
    let mut backend = WritableTestBackend::new(80, 24);
    let _ = backend.write(b"coverage").expect("write backend");
    Write::flush(&mut backend).expect("flush backend");
    Backend::flush(&mut backend).expect("flush backend");
    let _ = backend.get_cursor_position().expect("get cursor");
    backend
        .set_cursor_position(ratatui::layout::Position::new(1, 1))
        .expect("set cursor");
    backend.clear().expect("clear backend");
    let _ = backend.window_size().expect("window size");
    Ok(())
}

#[cfg(test)]
fn run_with_logs<F>(
    main_runner: F,
    action_log: &std::path::Path,
    error_log: &std::path::Path,
) -> Result<(), Box<dyn Error>>
where
    F: FnOnce() -> Result<(), Box<dyn Error>>,
{
    log_action_to(action_log, "=== FunnyFileManager Started ===");
    let result = main_runner();
    if let Err(ref error) = result {
        log_error_to(error_log, &format!("Main error: {error}"));
    }
    log_action_to(action_log, "=== FunnyFileManager Ended ===");
    result
}

pub(crate) fn run_with<F>(main_runner: F) -> Result<(), Box<dyn Error>>
where
    F: FnOnce() -> Result<(), Box<dyn Error>>,
{
    log_action("=== FunnyFileManager Started ===");
    let result = main_runner();
    if let Err(ref error) = result {
        log_error(&format!("Main error: {error}"));
    }
    log_action("=== FunnyFileManager Ended ===");
    result
}

pub fn run_app<B: Backend + Write>(terminal: &mut Terminal<B>, app: App) -> io::Result<()> {
    run_app_with_event_reader(terminal, app, event::read)
}

#[cfg(any(test, coverage, feature = "test-helpers"))]
mod test_backend;
#[cfg(any(test, coverage, feature = "test-helpers"))]
pub use test_backend::WritableTestBackend;

/// Control-flow signal from [`apply_action`]: keep looping, or exit cleanly.
/// Run one fallible user operation and surface any failure in the status bar.
///
/// Routine I/O failure — F3 on a binary file, F5 into a read-only directory,
/// Enter on a directory the user cannot read — must never leave the event loop:
/// `run_app`'s caller treats an `Err` as fatal, so a `?` here would quit the
/// file manager on a bad keypress. Only a failed terminal draw is fatal.
fn report(app: &mut App, operation: impl FnOnce(&mut App) -> io::Result<()>) {
    if let Err(error) = operation(app) {
        app.set_status(crate::status::Status::Error(error.to_string()));
    }
}

/// Apply a resolved [`Action`] to the application state, performing its effect.
///
/// The dispatch half of the Input→Action DSL: [`crate::action::resolve`] turns a
/// key into an [`Action`]; this turns that [`Action`] into an effect on `app`.
/// Returns [`Flow::Quit`] when the action should terminate the event loop.
///
/// Infallible by construction: every fallible effect goes through [`report`],
/// so a failing file operation can only ever reach the status bar, never the
/// event loop.
fn apply_action<B>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    action: Action,
    last_nav_time: &mut Instant,
    nav_debounce: Duration,
) where
    B: Backend + Write,
{
    log_action(&action_label(action));
    if app.view == ViewMode::Console {
        console_action(terminal, app, action);
        return;
    }

    match action {
        Action::Quit => app.lifecycle = Lifecycle::Quitting,
        Action::NavDown | Action::NavUp => {
            let now = Instant::now();
            if now.duration_since(*last_nav_time) >= nav_debounce {
                let panel = app.get_active_panel();
                if matches!(action, Action::NavDown) {
                    panel.next();
                } else {
                    panel.previous();
                }
                *last_nav_time = now;
            }
        }
        Action::Open => report(app, |app| app.get_active_panel().enter()),
        Action::SwitchPanel => app.switch_panel(),
        Action::SelectPanel(side) => app.active_panel = side,
        Action::First => app.get_active_panel().select(Some(0)),
        Action::Last => {
            let panel = app.get_active_panel();
            if !panel.entries.is_empty() {
                panel.select(Some(panel.entries.len() - 1));
            }
        }
        Action::PageUp | Action::PageDown => {
            // Page by exactly one screen, so the entry under the cursor moves to
            // the opposite edge of the panel the way it does in Far Manager.
            let height = terminal
                .size()
                .map_or(crate::ui::FALLBACK_TERMINAL_HEIGHT, |size| size.height);
            let page = crate::ui::panel_page_size(height);
            let panel = app.get_active_panel();
            if matches!(action, Action::PageDown) {
                panel.page_down(page);
            } else {
                panel.page_up(page);
            }
        }
        Action::DriveMenu(side) => app.set_overlay(Overlay::DriveSelection {
            side,
            selected_index: 0,
        }),
        Action::NewFile => app.set_overlay(Overlay::NewFileInput(String::new())),
        Action::ToggleHelp => {
            if matches!(app.overlay, Overlay::Help) {
                app.close_overlay();
            } else {
                app.set_overlay(Overlay::Help);
            }
        }
        Action::View => report(app, handle_view),
        Action::Edit => report(app, handle_edit_builtin),
        Action::EditExternal => report(app, |app| handle_edit(terminal, app)),
        Action::Copy => report(app, handle_copy),
        Action::Move => report(app, handle_move),
        Action::Mkdir => report(app, handle_mkdir),
        Action::DeleteConfirm => report(app, handle_delete_confirm),
        Action::McpStatus => report(app, crate::ops::open_mcp_status),
        Action::ToggleMark => {
            app.get_active_panel().toggle_mark_selected();
            app.get_active_panel().next();
        }
        Action::GoRoot => {
            // Through `navigate_to`, so the name-keyed marks cannot follow the
            // panel to the root and re-bind to `/bin`, `/tmp`, `/var`, ...
            let root = PathBuf::from(if cfg!(windows) { "C:\\" } else { "/" });
            report(app, |app| app.get_active_panel().navigate_to(root));
            app.quick_search.clear();
        }
        Action::SwapPanels => {
            app.swap_panels();
            app.set_status(crate::status::Status::PanelsSwapped);
        }
        Action::ToggleConsole => app.view = app.view.toggled(),
        Action::Refresh => match crate::utils::refresh_panels(app) {
            Ok(()) => app.set_status(crate::status::Status::PanelsRefreshed),
            Err(e) => app.set_status(crate::status::Status::RefreshFailed(e.to_string())),
        },
        Action::CommandLine => app.set_overlay(Overlay::CommandLine(CommandLine::default())),
        Action::CommandLineChar(ch) => open_command_line_with(app, ch),
        Action::QuickSearch(c) => {
            // The query rules live on `QuickSearch`; the panel owns the match.
            let query = app.quick_search.push(c).to_owned();
            if !app.get_active_panel().find_first_matching(&query)
                && app.quick_search.should_restart()
            {
                let restarted = app.quick_search.restart_with(c).to_owned();
                app.get_active_panel().find_first_matching(&restarted);
            }
        }
        Action::QuickSearchBackspace => {
            if let Some(query) = app.quick_search.backspace().map(str::to_owned) {
                app.get_active_panel().find_first_matching(&query);
            }
        }
        Action::QuickSearchClear => app.quick_search.clear(),
    }
}

/// Apply an action while `Ctrl+O` holds the panels off screen.
///
/// Far Manager's console mode is "show me the output *and let me keep
/// working*", so the command line still opens and still runs commands — their
/// output lands in the transcript right behind the prompt. The navigation keys
/// scroll that transcript instead of a panel nobody can see, and every action
/// that would touch the panels is swallowed: they are not on screen to act on,
/// and F8 silently deleting a file behind a full-screen console is exactly the
/// kind of thing the old modal `Overlay::Console` was protecting against.
fn console_action<B>(terminal: &mut Terminal<B>, app: &mut App, action: Action)
where
    B: Backend + Write,
{
    // The console is drawn full-screen, so one page is the terminal itself.
    let page = terminal
        .size()
        .map_or(crate::ui::FALLBACK_TERMINAL_HEIGHT, |size| size.height);
    let page = usize::from(page).max(1);

    match action {
        Action::Quit => app.lifecycle = Lifecycle::Quitting,
        Action::ToggleConsole => app.view = app.view.toggled(),
        Action::NavUp => app.console.scroll_up(1),
        Action::NavDown => app.console.scroll_down(1),
        Action::PageUp => app.console.scroll_up(page),
        Action::PageDown => app.console.scroll_down(page),
        Action::First => app.console.scroll_to_top(),
        Action::Last => app.console.scroll_to_bottom(),
        Action::CommandLine => app.set_overlay(Overlay::CommandLine(CommandLine::default())),
        Action::CommandLineChar(ch) => open_command_line_with(app, ch),
        _ => {}
    }
}

/// Open the command line already holding `ch`.
///
/// The character is *replayed through the overlay's own key handler* rather
/// than pushed into the buffer directly, so the first keystroke of a command
/// gets everything a later one does — the history ghost in particular. Two
/// insertion paths would be one too many, and the second would be the one
/// nobody remembers to keep in step.
fn open_command_line_with(app: &mut App, ch: char) {
    app.set_overlay(Overlay::CommandLine(CommandLine::default()));
    process_popup_input(app, KeyEvent::new(KeyCode::Char(ch), KeyModifiers::empty()));
}

fn dispatch_key<B>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    key: KeyEvent,
    last_nav_time: &mut Instant,
    nav_debounce: Duration,
) where
    B: Backend + Write,
{
    if app.is_overlay_active() {
        process_popup_input(app, key);
        if !app.lifecycle.is_running() {
            log_action("Quitting from overlay");
        }
        return;
    }

    apply_action(terminal, app, resolve(key), last_nav_time, nav_debounce);
}

pub fn run_app_with_event_reader<B, F>(
    terminal: &mut Terminal<B>,
    mut app: App,
    mut read_event: F,
) -> io::Result<()>
where
    B: Backend + Write,
    F: FnMut() -> io::Result<Event>,
{
    let mut last_nav_time = Instant::now();
    #[cfg(coverage)]
    let nav_debounce = Duration::from_millis(250);
    #[cfg(not(coverage))]
    let nav_debounce = Duration::from_millis(0);

    log_action(&format!(
        "Starting event loop with {} entries in left panel",
        app.left_panel.entries.len()
    ));

    while app.lifecycle.is_running() {
        // ratatui 0.30 made `Backend::Error` an associated type (io::Error for
        // crossterm, Infallible for TestBackend). Convert via Display so this stays
        // generic over B without leaking a `From` bound onto every caller.
        terminal
            .draw(|f| ui(f, &mut app))
            .map_err(|e| io::Error::other(e.to_string()))?;

        let event = read_event()?;
        if let Event::Key(key) = event {
            // Only process Press events, ignore Release/Repeat to avoid double navigation on Windows
            if key.kind != KeyEventKind::Press {
                continue;
            }

            log_action(&format!(
                "Key event: {} (modifiers: {:?})",
                key_label(key.code),
                key.modifiers
            ));

            dispatch_key(terminal, &mut app, key, &mut last_nav_time, nav_debounce);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_command_line_help;
#[cfg(test)]
mod tests_dispatch_errors;
#[cfg(test)]
mod tests_event_loop_keys;
#[cfg(test)]
mod tests_event_loop_misc;
#[cfg(test)]
mod tests_event_loop_toggles;
#[cfg(test)]
mod tests_fkey_dialogs;
#[cfg(test)]
mod tests_marking_and_panels;
#[cfg(test)]
mod tests_quick_search_debounce;
#[cfg(test)]
mod tests_quick_search_nav;
#[cfg(test)]
mod tests_run_lifecycle;
