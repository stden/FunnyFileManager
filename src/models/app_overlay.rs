//! Purpose: `App`'s overlay accessors, and the dialog and viewport bookkeeping around them.

use std::path::PathBuf;

use crate::lineedit::LineEditor;

use super::{App, Editor, Overlay, TaskList, Viewer, Viewport};

mod focus;

impl App {
    // === Overlay helpers (new enum-based API) ===

    pub const fn is_overlay_active(&self) -> bool {
        self.overlay.is_active()
    }

    pub fn close_overlay(&mut self) {
        self.overlay.close();
    }

    /// Returns a reference to the current overlay.
    pub const fn get_overlay(&self) -> &Overlay {
        &self.overlay
    }

    /// Returns a mutable reference to the current overlay.
    pub const fn get_overlay_mut(&mut self) -> &mut Overlay {
        &mut self.overlay
    }

    /// Convenience: returns the active Viewer if the overlay is Viewer.
    pub const fn get_viewer(&self) -> Option<&Viewer> {
        if let Overlay::Viewer(v) = &self.overlay {
            Some(v)
        } else {
            None
        }
    }

    pub const fn get_viewer_mut(&mut self) -> Option<&mut Viewer> {
        if let Overlay::Viewer(v) = &mut self.overlay {
            Some(v)
        } else {
            None
        }
    }

    /// Convenience: returns the active Editor if the overlay is Editor.
    pub const fn get_editor(&self) -> Option<&Editor> {
        if let Overlay::Editor(e) = &self.overlay {
            Some(e)
        } else {
            None
        }
    }

    /// The task list, when it is what is open.
    pub fn task_list_mut(&mut self) -> Option<&mut TaskList> {
        if let Overlay::Tasks(list) = &mut self.overlay {
            Some(list)
        } else {
            None
        }
    }

    pub const fn get_editor_mut(&mut self) -> Option<&mut Editor> {
        if let Overlay::Editor(e) = &mut self.overlay {
            Some(e)
        } else {
            None
        }
    }

    /// Convenience: returns the vault session if the overlay is Kdbx.
    #[cfg(feature = "keepass")]
    pub const fn kdbx_session(&self) -> Option<&crate::kdbx::KdbxSession> {
        if let Overlay::Kdbx(overlay) = &self.overlay {
            Some(&overlay.0)
        } else {
            None
        }
    }

    #[cfg(feature = "keepass")]
    pub const fn kdbx_session_mut(&mut self) -> Option<&mut crate::kdbx::KdbxSession> {
        if let Overlay::Kdbx(overlay) = &mut self.overlay {
            Some(&mut overlay.0)
        } else {
            None
        }
    }

    pub fn get_command_line(&self) -> Option<&str> {
        if let Overlay::CommandLine(state) = &self.overlay {
            Some(state.line.text())
        } else {
            None
        }
    }

    pub fn get_new_file_input(&self) -> Option<&str> {
        if let Overlay::NewFileInput(s) = &self.overlay {
            Some(s.text())
        } else {
            None
        }
    }

    pub const fn get_delete_confirm(&self) -> Option<&Vec<PathBuf>> {
        if let Overlay::DeleteConfirm(files) = &self.overlay {
            Some(files)
        } else {
            None
        }
    }

    pub fn take_delete_confirm(&mut self) -> Option<Vec<PathBuf>> {
        match std::mem::take(&mut self.overlay) {
            Overlay::DeleteConfirm(paths) => Some(paths),
            other => {
                self.overlay = other;
                None
            }
        }
    }

    pub const fn get_new_file_input_mut(&mut self) -> Option<&mut LineEditor> {
        if let Overlay::NewFileInput(s) = &mut self.overlay {
            Some(s)
        } else {
            None
        }
    }

    /// The editable line, when the command-line overlay is open.
    ///
    /// Returns the [`LineEditor`], never a `&mut String`: handing out the raw
    /// buffer would let a caller shorten the text under a caret that then
    /// points past its end, which is exactly the invariant `LineEditor` exists
    /// to hold.
    pub const fn get_command_line_mut(&mut self) -> Option<&mut LineEditor> {
        if let Overlay::CommandLine(state) = &mut self.overlay {
            Some(&mut state.line)
        } else {
            None
        }
    }

    /// Move the menu without going through [`Self::set_overlay`], which
    /// would recentre a dialog that is not there and is not what walking a
    /// menu means.
    ///
    /// Showing a heading never changes the active panel: walking the bar past
    /// Left or Right, or leaving it with Esc, must leave Files and Commands
    /// acting on the panel the user was in. Choosing a row of a panel's menu
    /// is what makes that panel active ([`crate::ops::menu::activate_row`]).
    pub fn set_menu(&mut self, menu: crate::menu::Menu) {
        self.overlay = Overlay::Menu(menu);
    }

    /// Replaces the current overlay with a new one.
    /// Only one overlay can be active at a time (enforced by the type system).
    pub fn set_overlay(&mut self, overlay: Overlay) {
        if matches!(overlay, Overlay::Help) {
            self.help_top = 0;
        }
        self.overlay = overlay;
        // A new dialog opens centred. Inheriting wherever the last one was
        // pushed to would put it somewhere the user never chose, and a dialog
        // that appears off to one side reads as a rendering fault.
        self.dialog_mouse.reset();
    }

    /// Record the terminal's height, and keep an open viewer or task list
    /// within it.
    ///
    /// The viewer's scroll limit moves with the height: a terminal that grows
    /// taller after End would otherwise leave the old top row in place with
    /// blank rows under the last line, and the next Down would jump the view
    /// *up* to the new limit. Far re-anchors on resize; so does this, here, so
    /// no caller can record a height without the clamp that goes with it. The
    /// task list's page shrinks with the terminal too, and its cursor must
    /// stay on screen.
    pub fn set_viewport(&mut self, viewport: Viewport) {
        self.viewport = viewport;
        match &mut self.overlay {
            Overlay::Viewer(viewer) => viewer.clamp(Viewer::page_for(viewport)),
            Overlay::Tasks(list) => list.clamp_to(viewport),
            // Every other overlay is laid out afresh on each frame.
            _ => {}
        }
    }
}
