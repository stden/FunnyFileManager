//! Unit tests: F5 / F6 / F8 must act on the link, never through it.
//!
//! `copy_dir_all` recreates *nested* links, but `copy_entry` — the entry point
//! F5 and F6 actually use — branched on `Path::is_dir`, which resolves the link.
//! A panel entry `logs -> /var/log` was therefore copied as the whole target
//! tree, and the move fallback deleted the user's link afterwards.

#![cfg(unix)]

use std::os::unix::fs::symlink;

use super::*;

use crate::models::{Panel, PanelSide};

use tempfile::TempDir;

/// Two panels rooted at `left` / `right`, left active — the F5/F6/F8 setup.
fn app_with_panels(left: &Path, right: &Path) -> io::Result<App> {
    let mut app = App::new()?;
    app.left_panel = Panel::new(left.to_path_buf())?;
    app.right_panel = Panel::new(right.to_path_buf())?;
    app.active_panel = PanelSide::Left;
    Ok(app)
}

/// Select `name` in the active panel, the way the cursor would.
fn select(app: &mut App, name: &str) {
    let panel = app.get_active_panel();
    let index = panel
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .expect("test setup: entry must be listed");
    panel.select(Some(index));
}

/// A directory the link can point at, holding one recognisable file.
fn make_target_tree(root: &Path) -> io::Result<PathBuf> {
    let target = root.join("target");
    fs::create_dir(&target)?;
    fs::write(target.join("inside.txt"), b"payload")?;
    Ok(target)
}

mod tests_link_copies;
mod tests_link_removal;
mod tests_renames;
