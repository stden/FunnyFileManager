//! Purpose: Start an `App` on an injected settings file, as startup does with
//! the one under the config home, so integration tests never touch the
//! user's own settings.

use std::path::PathBuf;

use crate::models::App;

/// Load the Options pages from `path` the way startup loads them: a failure
/// goes to the status line, and the file stays the target of Save setup.
pub fn restore_settings(app: &mut App, path: PathBuf) {
    crate::ops::menu::settings::restore(app, path);
}
