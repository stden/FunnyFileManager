//! Unit tests: Shift+F2 unpacks every selected archive into the other panel,
//! says what it did in the status bar and the console, and refreshes.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

use super::{extract_archive, is_archive, open_extract, run_extract};
use crate::models::{App, ExtractField, Overlay, Panel, PanelSide};
use crate::status::Status;

/// A folder holding one recognisable file, packed as `name` next to it.
fn pack(root: &Path, name: &str) -> PathBuf {
    pack_files(root, name, &[("payload/nested/hello.txt", "hello")])
}

/// `files` (relative path, contents) packed as `name` in `root`: `tar` for
/// the tar family, Python's `zipfile` for a zip. Nothing else is left behind.
pub(super) fn pack_files(root: &Path, name: &str, files: &[(&str, &str)]) -> PathBuf {
    let source = TempDir::new().expect("source dir");
    for (path, contents) in files {
        let file = source.path().join(path);
        fs::create_dir_all(file.parent().expect("parent")).expect("source dirs");
        fs::write(&file, contents).expect("source file");
    }
    let tops: std::collections::BTreeSet<&str> = files
        .iter()
        .filter_map(|(path, _)| path.split('/').next())
        .collect();
    let archive = root.join(name);
    let is_zip = Path::new(name)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"));
    let mut packer = if is_zip {
        let mut python = Command::new("python3");
        python.args(["-m", "zipfile", "-c"]);
        python
    } else {
        let mut tar = Command::new("tar");
        tar.arg("-caf");
        tar
    };
    let status = packer
        .arg(&archive)
        .args(tops)
        .current_dir(source.path())
        .status();
    assert!(status.expect("packer runs").success(), "packing {name}");
    archive
}

/// Whether `tool` is installed. Only the start is asked for: `unzip` knows
/// no `--version` and exits 10 on it, which once skipped every zip test.
pub(super) fn has(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

/// Two panels: `left` (active, holding the archives) and `right` (destination).
fn app_between(left: &Path, right: &Path) -> App {
    let mut app = App::new().expect("test app");
    app.left_panel = Panel::new(left.to_path_buf()).expect("left panel");
    app.right_panel = Panel::new(right.to_path_buf()).expect("right panel");
    app.active_panel = PanelSide::Left;
    app
}

fn select(app: &mut App, name: &str) {
    let panel = app.get_active_panel();
    let index = panel
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .expect("entry listed");
    panel.select(Some(index));
}

mod tests_open;
mod tests_refusals;
mod tests_run;
mod tests_unpacking;
