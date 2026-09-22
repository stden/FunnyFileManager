//! Restore the two panel directories at startup and checkpoint navigation.

use std::{io, path::Path};

use crate::{
    history::Persistence,
    models::{App, Panel},
    panel_session::PanelSession,
};

/// Load the last panel directories, falling back independently to `fallback`.
/// `SessionOnly` leaves the user's saved session untouched.
pub fn load(fallback: &Path, persistence: Persistence) -> io::Result<App> {
    let session = PanelSession::load(persistence);
    let paths = session.paths();
    let left = load_panel(paths.map(|paths| paths.left.as_path()), fallback)?;
    let right = load_panel(paths.map(|paths| paths.right.as_path()), fallback)?;
    let mut app = App::with_panels(left, right);
    app.panel_session = session;
    Ok(app)
}

fn load_panel(saved: Option<&Path>, fallback: &Path) -> io::Result<Panel> {
    if let Some(path) = saved.filter(|path| path.is_absolute()) {
        match Panel::new(path.to_owned()) {
            Ok(panel) => return Ok(panel),
            Err(error) => crate::diagnostics::log_error(&format!("restoring panel: {error}")),
        }
    }
    Panel::new(fallback.to_owned())
}

/// A failed checkpoint must not prevent navigation or a clean exit.
pub(crate) fn checkpoint(app: &mut App) {
    if let Err(error) = app
        .panel_session
        .save(&app.left_panel.path, &app.right_panel.path)
    {
        crate::diagnostics::log_error(&format!("saving panel session: {error}"));
    }
}

#[cfg(test)]
mod tests;
