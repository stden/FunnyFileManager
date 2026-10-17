//! Purpose: `App`, the session state around two panels, and its constructors.
//! Choosing and swapping panels lives in `app/panels.rs`.

use std::io;

use crate::console::ConsoleLog;
use crate::quicksearch::QuickSearch;

use super::{
    settings, Bookmarks, ClipboardSlot, FolderHistory, Lifecycle, Overlay, Panel, PanelSide,
    PanelsShown, TaskHostSlot, ViewHistory, ViewMode, Viewport, WslRequests,
};

mod live_input;
mod panels;

pub use live_input::{LiveInput, ProgramPause};

/// Main application state: two panels, overlays, bookmarks, etc.
#[derive(Clone)]
pub struct App {
    pub left_panel: Panel,
    pub right_panel: Panel,
    pub active_panel: PanelSide,

    /// What is drawn behind the overlays: the panels, or the console
    /// transcript that `Ctrl+O` reveals. Orthogonal to `overlay` on purpose —
    /// see [`ViewMode`].
    pub view: ViewMode,

    /// Single source of truth for all modal / overlay state.
    /// Using one enum enforces mutual exclusivity and makes illegal states
    /// (two overlays open at once) impossible at compile time.
    pub overlay: Overlay,

    pub bookmarks: Bookmarks,
    pub settings: settings::Settings,
    pub lifecycle: Lifecycle,
    pub status: crate::status::Status,
    /// Whether the message is pending, drawn at a cursor, or expired from the
    /// info row. The command prompt keeps the underlying outcome in `status`.
    pub(crate) status_display: crate::status::StatusDisplay,
    pub quick_search: QuickSearch,

    /// Executed `:` commands, session-wide, persisted per the durability gate.
    pub history: crate::history::History,

    /// Far-style console transcript: every executed command and what it
    /// printed, drawn as the layer *behind* the panels and revealed by
    /// `Ctrl+O`. In memory only — unlike `history`, nothing here is ever
    /// written to disk, so a token in a shell command dies with the process.
    pub console: ConsoleLog,
    /// Input-to-frame timings; saving belongs to production bootstrap.
    pub performance: crate::performance::Performance,
    pub(crate) package_update: Option<crate::packages::UpdatePlan>,
    pub(crate) git_pull_root: Option<std::path::PathBuf>,
    /// Where the dialog on screen was drawn, and how far it has been dragged.
    /// Written by the renderer, read by the next click.
    pub dialog_mouse: crate::mouse::DialogMouse,
    /// The menu row chosen, whose action the dispatcher runs once the
    /// drop-down has left the screen, so a command that opens a window of
    /// its own is not fighting the menu for it. The row rather than its
    /// action: state names the menu, never the action layer above it.
    pub pending_row: Option<crate::menu::Entry>,
    /// Which panels are on screen — `Ctrl+F1` / `Ctrl+F2` hide one side.
    pub panels_shown: PanelsShown,
    /// The Options menu's pages. Defaults here; the production entry point
    /// replaces them with the saved file, the way it does `history`.
    pub config: crate::config::Settings,
    /// The keys in force on every screen. The defaults here, so that no
    /// test reads the user's `ffm.yml`; the production entry point replaces
    /// them with the file's.
    pub keymap: crate::keymap::Keymap,
    /// Files F3 and F4 opened — `Alt+F11`. Session-only until bootstrap loads
    /// the persisted list.
    pub view_history: ViewHistory,
    /// Directories the panels have shown — `Alt+F12`. Filled from the panels'
    /// pending visits by [`Self::record_folder_visits`].
    pub folder_history: FolderHistory,
    /// Last displayed directories; bootstrap explicitly enables persistence.
    pub(crate) panel_session: crate::panel_session::PanelSession,
    /// `NetBox`'s stored sessions. Session-only here; bootstrap loads the
    /// file, the way it does the settings, so no test reads the user's own.
    pub sessions: crate::sessions::SessionStore,
    /// The terminal's height, refreshed at dispatch time — the one answer for
    /// every consumer that pages by the screen (panels, console, viewer).
    /// Stored through [`Self::set_viewport`], which keeps an open viewer
    /// within it.
    pub viewport: Viewport,

    /// Session clipboard for editor text and optional vault secrets.
    /// The backend stays alive across overlays; constructors leave it absent
    /// and production bootstrap installs the real adapter.
    pub clipboard: ClipboardSlot,
    /// What only live input gets; see [`LiveInput`].
    pub live: LiveInput,
    #[cfg(feature = "keepass")]
    pub(crate) vault_cache: crate::kdbx::cache::SessionCache,
    /// Requests awaiting interpretation by the background WSL worker.
    pub wsl_requests: WslRequests,
    /// The process host the task list reads and controls processes through.
    /// Constructors leave it empty — the list then falls back to the text
    /// report — and the production entry point installs the platform's.
    pub task_host: TaskHostSlot,
}

impl App {
    /// Scan the launch directory and build session-only application state.
    ///
    /// `history` starts empty (`History::default()`, `path: None`, `save()` a
    /// permanent no-op — see `history.rs`). The real on-disk history is loaded
    /// exactly once at the production entry point in `bootstrap`,
    /// never here: keeping persistence out of `App::new()` means no test — unit or
    /// integration, present or future — can pollute the user's real
    /// `~/.local/share/ffm/history` merely by constructing an `App`.
    pub fn new() -> io::Result<Self> {
        let current_dir = std::env::current_dir()?;
        Ok(Self::with_panels(
            Panel::new(current_dir.clone())?,
            Panel::new(current_dir)?,
        ))
    }

    /// Session state around two panels already loaded — every other field at
    /// its default, and nothing persistent. The one place the defaults are
    /// listed, so a new field cannot be initialised differently by `new()`
    /// and by a test that brings its own panels.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "one line per App field: splitting it would list the defaults in two places"
    )]
    pub fn with_panels(left_panel: Panel, right_panel: Panel) -> Self {
        Self {
            left_panel,
            right_panel,
            active_panel: PanelSide::Left,
            view: ViewMode::default(),
            overlay: Overlay::default(),
            bookmarks: Bookmarks::default(),
            settings: settings::Settings::default(),
            lifecycle: Lifecycle::default(),
            status: crate::status::Status::KeyHints,
            status_display: crate::status::StatusDisplay::default(),
            quick_search: QuickSearch::default(),
            history: crate::history::History::default(),
            console: ConsoleLog::default(),
            performance: crate::performance::Performance::default(),
            package_update: None,
            git_pull_root: None,
            dialog_mouse: crate::mouse::DialogMouse::default(),
            pending_row: None,
            panels_shown: PanelsShown::default(),
            config: crate::config::Settings::default(),
            keymap: crate::keymap::Keymap::default(),
            view_history: ViewHistory::default(),
            folder_history: FolderHistory::default(),
            panel_session: crate::panel_session::PanelSession::default(),
            sessions: crate::sessions::SessionStore::default(),
            viewport: Viewport::default(),
            // Default is backend-less: installing the OS clipboard here would
            // put an X11/Wayland connection inside every test's `App::new()`.
            // The production entry point installs the real backend.
            clipboard: ClipboardSlot::default(),
            live: LiveInput::default(),
            #[cfg(feature = "keepass")]
            vault_cache: crate::kdbx::cache::SessionCache::default(),
            wsl_requests: WslRequests::default(),
            task_host: TaskHostSlot::default(),
        }
    }

    /// Upstream spelling of [`Self::with_panels`], kept so tests and callers
    /// written against either name compile against one implementation.
    #[must_use]
    pub fn from_panels(left_panel: Panel, right_panel: Panel) -> Self {
        Self::with_panels(left_panel, right_panel)
    }
}
