//! Unit tests: Ctrl+A reads the selection into the dialog, and Set writes the
//! dialog back — to every marked entry, into subfolders when asked, and with
//! an honest count when something refuses.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::{apply, open, stamp};
use crate::models::{App, AttributesDialog, AttributesField, Flag, Overlay, Panel};

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

fn mtime(path: &Path) -> String {
    crate::models::format_time(
        fs::metadata(path)
            .expect("metadata")
            .modified()
            .expect("mtime"),
    )
}

#[test]
fn process_subfolders_skips_directory_links_across_platforms() {
    let temp = TempDir::new().expect("temp");
    let dir = temp.path().join("folder");
    let outside = temp.path().join("outside");
    let inside = dir.join("inside.txt");
    let outside_file = outside.join("kept.txt");
    fs::create_dir_all(&dir).expect("mkdir folder");
    fs::create_dir_all(&outside).expect("mkdir outside");
    fs::write(&inside, "in").expect("write inside");
    fs::write(&outside_file, "out").expect("write outside");
    let before = mtime(&outside_file);
    crate::test_support::make_dir_link(&outside, &dir.join("link_out")).expect("make dir link");

    let mut app = app_at(temp.path());
    app.left_panel.select_named("folder");
    open(&mut app);
    let mut dlg = dialog(&app).clone();
    dlg.modified.current = "2001-02-03 04:05:06".to_owned();
    dlg.process_subfolders = true;
    app.close_overlay();

    apply(&mut app, &dlg);

    assert_eq!(app.status_text(), "Attributes set on 2 object(s)");
    assert_eq!(mtime(&inside), "2001-02-03 04:05:06");
    assert_eq!(mtime(&outside_file), before);
}

mod applying;
mod dialog_opening;
#[cfg(unix)]
mod recursion;
mod stamping;
