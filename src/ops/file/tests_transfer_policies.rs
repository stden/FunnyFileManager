//! Unit tests: what the F5/F6 dialog's options actually do to the filesystem.
//!
//! Every control in that dialog is a promise about bytes on disk — "skip what
//! is already there", "keep the timestamps", "copy the permissions" — and a
//! promise nothing checks is the kind that is quietly wrong for months. The
//! dialog's *state* is tested next door in `copy::tests`; this is about the
//! transfer it ends in.

use super::*;

use crate::copy::{CopyPrompt, ExistingFiles, Purpose};
use crate::models::{FileName, Panel, PanelSide};
use crate::status::{Landed, Status};
use crate::utils::CopyRules;

use tempfile::TempDir;

/// Two panels, left active and holding `sources`, right empty.
pub(super) fn app_with(sources: &[(&str, &[u8])]) -> (App, TempDir, TempDir) {
    let from = TempDir::new().expect("test setup");
    let to = TempDir::new().expect("test setup");
    for (name, body) in sources {
        fs::write(from.path().join(name), body).expect("test setup");
    }
    let app = app_between(from.path(), to.path());
    (app, from, to)
}

/// Two panels on directories that already exist, left active on `from`.
pub(super) fn app_between(from: &Path, to: &Path) -> App {
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(from.to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(to.to_path_buf()).expect("test setup");
    app.active_panel = PanelSide::Left;
    app
}

/// Mark `name` in the active panel, which is what the transfer acts on.
pub(super) fn mark(app: &mut App, name: &str) {
    app.get_active_panel()
        .marked
        .insert(FileName::new(name.to_owned()));
}

/// The dialog as it would be filled in, aimed at `destination`.
pub(super) fn settings(destination: &Path) -> CopyPrompt {
    CopyPrompt::new("marked", destination)
}

/// Make a FIFO at `path`, or `None` where the platform will not have one.
#[cfg(unix)]
pub(super) fn make_fifo(path: &Path) -> Option<PathBuf> {
    crate::test_support::make_fifo(path).ok()?;
    Some(path.to_path_buf())
}

#[cfg(unix)]
mod tests_dangling_links;
mod tests_existing_files;
mod tests_folders_and_moves;
#[cfg(unix)]
mod tests_link_contents;
mod tests_onto_itself;
mod tests_refusals;
mod tests_special_entries;
