//! Restore the two panel directories at startup and checkpoint navigation.

use std::{
    io,
    path::{Path, PathBuf},
};

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

/// Where a panel opens when the saved session has nothing usable for it.
///
/// The launch directory, as `getcwd` reports it. When that folder was deleted
/// before ffm started, `getcwd` fails, but the shell's `PWD` still names it:
/// the nearest folder above it that exists is used instead, then the home
/// folder. Only with none of them known is the original failure reported.
pub fn start_directory(
    current: io::Result<PathBuf>,
    shell_directory: Option<PathBuf>,
    home: Option<PathBuf>,
) -> io::Result<PathBuf> {
    let error = match current {
        Ok(directory) => return Ok(directory),
        Err(error) => error,
    };
    [shell_directory, home]
        .into_iter()
        .flatten()
        .filter(|directory| directory.is_absolute())
        .find_map(|directory| crate::directory::nearest_existing(&directory).map(Path::to_path_buf))
        .ok_or_else(|| {
            let reason = format!("the launch folder is gone and no home folder is set: {error}");
            io::Error::new(error.kind(), reason)
        })
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
