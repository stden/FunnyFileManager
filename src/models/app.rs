//! Purpose: `App`, the session state around two panels: constructors and panel selection.

use std::io;

use crate::console::ConsoleLog;
use crate::quicksearch::QuickSearch;

use super::{
    settings, Bookmarks, ClipboardSlot, FolderHistory, Lifecycle, Overlay, Panel, PanelSide,
    PanelsShown, ViewHistory, ViewMode, Viewport, WslRequests,
};

/// What happens on the terminal after a program started from a panel ends.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProgramPause {
    /// The panels return at once.
    #[default]
    None,
    /// The output stays on screen until a key is pressed.
    UntilKey,
}

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
    /// Where the dialog on screen was drawn, and how far it has been dragged.
    /// Written by the renderer, read by the next click.
    pub dialog_mouse: crate::mouse::DialogMouse,
    /// A keystroke the menu chose, dispatched once the drop-down has closed, so
    /// a command that opens a window of its own is not fighting the menu for
    /// the screen.
    pub pending_key: Option<crate::menu::Shortcut>,
    /// What the menu chose, to be run once the menu has closed. A row with a
    /// key runs by pressing that key; a row without one carries its command.
    /// Either way the drop-down leaves the screen first, so a command that
    /// Which panels are on screen — `Ctrl+F1` / `Ctrl+F2` hide one side.
    pub panels_shown: PanelsShown,
    /// The Options menu's pages. Defaults here; the production entry point
    /// replaces them with the saved file, the way it does `history`.
    pub config: crate::config::Settings,
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
    /// Whether a finished program's output waits for a key before the panels
    /// return. Off by default: nobody is at a test's terminal to press one.
    /// The production entry point turns it on for live input.
    pub program_pause: ProgramPause,
    #[cfg(feature = "keepass")]
    pub(crate) vault_cache: crate::kdbx::cache::SessionCache,
    /// Requests awaiting interpretation by the background WSL worker.
    pub wsl_requests: WslRequests,
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
            dialog_mouse: crate::mouse::DialogMouse::default(),
            pending_key: None,
            panels_shown: PanelsShown::default(),
            config: crate::config::Settings::default(),
            view_history: ViewHistory::default(),
            folder_history: FolderHistory::default(),
            panel_session: crate::panel_session::PanelSession::default(),
            sessions: crate::sessions::SessionStore::default(),
            viewport: Viewport::default(),
            // Default is backend-less: installing the OS clipboard here would
            // put an X11/Wayland connection inside every test's `App::new()`.
            // The production entry point installs the real backend.
            clipboard: ClipboardSlot::default(),
            program_pause: ProgramPause::default(),
            #[cfg(feature = "keepass")]
            vault_cache: crate::kdbx::cache::SessionCache::default(),
            wsl_requests: WslRequests::default(),
        }
    }

    /// Upstream spelling of [`Self::with_panels`], kept so tests and callers
    /// written against either name compile against one implementation.
    #[must_use]
    pub fn from_panels(left_panel: Panel, right_panel: Panel) -> Self {
        Self::with_panels(left_panel, right_panel)
    }

    /// The panels on screen this frame: `Ctrl+F1` / `Ctrl+F2` hide one, and
    /// a fullscreen view mode on the active panel takes the other's half too.
    /// The one answer for the renderer and for the mouse.
    #[must_use]
    pub fn panels_on_screen(&self) -> PanelsShown {
        let fullscreen = self.get_active_panel_ref().view.is_fullscreen();
        self.panels_shown
            .with_fullscreen(self.active_panel, fullscreen)
    }

    /// Give both panels the description-file names the settings hold,
    /// re-listing a panel whose names changed so its descriptions come from
    /// the file now named. The panels are listed before the settings are
    /// loaded, which is why this is a step of its own; a failed re-read keeps
    /// the last snapshot, as every refresh does.
    pub fn adopt_description_names(&mut self) {
        let names = self.config.descriptions.names.clone();
        for panel in [&mut self.left_panel, &mut self.right_panel] {
            if panel.description_names == names {
                continue;
            }
            panel.description_names.clone_from(&names);
            if let Err(error) = panel.load_entries() {
                crate::diagnostics::log_error(&format!(
                    "re-reading {} for its descriptions: {error}",
                    panel.path.display()
                ));
            }
        }
    }

    /// Move the panels' pending visits into the folder history.
    ///
    /// A panel changes directory in one place, `Panel::apply_navigation`, but
    /// it has no `App` in reach there; so it leaves a note and the event loop
    /// calls this once per event, whatever key or command did the navigating.
    pub fn record_folder_visits(&mut self) {
        for panel in [&mut self.left_panel, &mut self.right_panel] {
            if let Some(path) = panel.pending_visit.take() {
                self.folder_history.touch(&path);
            }
        }
    }

    pub const fn get_active_panel(&mut self) -> &mut Panel {
        match self.active_panel {
            PanelSide::Left => &mut self.left_panel,
            PanelSide::Right => &mut self.right_panel,
        }
    }

    pub const fn get_active_panel_ref(&self) -> &Panel {
        match self.active_panel {
            PanelSide::Left => &self.left_panel,
            PanelSide::Right => &self.right_panel,
        }
    }

    pub const fn get_inactive_panel(&mut self) -> &mut Panel {
        match self.active_panel {
            PanelSide::Left => &mut self.right_panel,
            PanelSide::Right => &mut self.left_panel,
        }
    }

    pub const fn get_inactive_panel_ref(&self) -> &Panel {
        match self.active_panel {
            PanelSide::Left => &self.right_panel,
            PanelSide::Right => &self.left_panel,
        }
    }

    /// The panel on `side`, for callers that name a side rather than "the
    /// active one" — the Left and Right menus, and the panel toggles.
    pub const fn panel(&self, side: PanelSide) -> &Panel {
        match side {
            PanelSide::Left => &self.left_panel,
            PanelSide::Right => &self.right_panel,
        }
    }

    pub const fn panel_mut(&mut self, side: PanelSide) -> &mut Panel {
        match side {
            PanelSide::Left => &mut self.left_panel,
            PanelSide::Right => &mut self.right_panel,
        }
    }

    /// Make `side` the active panel — if it is on screen. A cursor on a
    /// panel nobody can see would act on files nobody can see.
    pub const fn select_panel(&mut self, side: PanelSide) {
        if self.panels_shown.shows(side) {
            self.active_panel = side;
        }
    }

    pub const fn switch_panel(&mut self) {
        self.active_panel = match self.active_panel {
            PanelSide::Left => PanelSide::Right,
            PanelSide::Right => PanelSide::Left,
        };
    }

    /// Swap the contents (paths) of left and right panels (Ctrl+U)
    pub const fn swap_panels(&mut self) {
        std::mem::swap(&mut self.left_panel, &mut self.right_panel);
    }
}
