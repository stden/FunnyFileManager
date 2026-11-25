//! Start image viewing from the panel's already known entry type.

use std::path::PathBuf;

use crate::models::{App, Overlay, Viewer};
use crate::status::Status;

/// Avoid another filesystem stat for a selected image: the current listing
/// already distinguishes files from directories and links to directories.
pub(super) fn selected(app: &App) -> Option<PathBuf> {
    let panel = app.get_active_panel_ref();
    if panel.get_selected_entry()?.kind.is_dir() {
        return None;
    }
    panel
        .selected_path()
        .filter(|path| crate::images::is_supported_path(path))
}

pub(super) fn open(app: &mut App, selected: PathBuf) {
    app.set_status(Status::Viewing(selected.display().to_string()));
    app.view_history.touch(&selected);
    app.set_overlay(Overlay::Viewer(Viewer::image(selected)));
}
