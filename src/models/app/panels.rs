//! Purpose: `App`'s two panels: which is active, which are on screen, reaching
//! one by side, switching and swapping them, and the bookkeeping both share.

use super::{App, Panel, PanelSide, PanelsShown};

#[cfg(test)]
mod tests_adopted_names;

impl App {
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
