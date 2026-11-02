//! Restore the two panel directories at startup and checkpoint navigation.

use std::{
    io,
    path::{Path, PathBuf},
};

use crate::{
    file_write::SyncPolicy,
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
        // Strip any `\\?\` verbatim prefix that a previous session may have
        // written (before normalize was applied at navigation time).
        let path = crate::path_display::normalize(path.to_path_buf());
        match Panel::new(path) {
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

/// The checkpoint taken before each redrawn frame, written without an fsync so
/// that the frame showing a new folder does not wait for the disk.
pub(crate) fn checkpoint(app: &mut App) {
    save_panels(app, SyncPolicy::Skip);
}

/// The exit save. The new contents are fsynced before the rename, but nothing
/// syncs the rename itself, so a power loss can still roll the session back to
/// the previous file.
pub(crate) fn flush(app: &mut App) {
    save_panels(app, SyncPolicy::Always);
}

/// A failed save must not prevent navigation or a clean exit.
fn save_panels(app: &mut App, policy: SyncPolicy) {
    let saved = app
        .panel_session
        .save(&app.left_panel.path, &app.right_panel.path, policy);
    if let Err(error) = saved {
        crate::diagnostics::log_error(&format!("saving panel session: {error}"));
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_flush;
