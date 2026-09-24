//! Unit tests: Ctrl+A reads the selection into the dialog, and Set writes the
//! dialog back — to every marked entry, into subfolders when asked, and with
//! an honest count when something refuses.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::{apply, open, stamp};
use crate::models::{App, AttributesDialog, Overlay, Panel};
#[cfg(unix)]
use crate::models::{AttributesField, Flag};

fn app_at(dir: &Path) -> App {
    let mut app = App::new().expect("test app");
    app.left_panel = Panel::new(dir.to_path_buf()).expect("panel");
    app
}

fn dialog(app: &App) -> &AttributesDialog {
    let Overlay::Attributes(dialog) = &app.overlay else {
        panic!("the attributes dialog is open");
    };
    dialog
}

/// Space until the box reads `[x]` — one press for a file, two for a
/// three-state box that opened on `[ ]`.
#[cfg(unix)]
fn set_flag(dialog: &mut AttributesDialog, flag: Flag) {
    let index = dialog
        .flags
        .iter()
        .position(|it| it.flag == flag)
        .expect("platform flag");
    dialog.field = AttributesField::Flag(index);
    while dialog.flags[index].current != crate::models::Check::Set {
        dialog.toggle();
    }
}

#[cfg(unix)]
fn mode(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).expect("metadata").permissions().mode() & 0o7777
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("chmod");
}

#[cfg(unix)]
fn mtime(path: &Path) -> String {
    crate::models::format_time(
        fs::metadata(path)
            .expect("metadata")
            .modified()
            .expect("mtime"),
    )
}

mod applying;
mod dialog_opening;
#[cfg(unix)]
mod recursion;
mod stamping;
