//! Purpose: Ctrl+A end to end through the public API — the dialog opens on
//! the marked entries, the keys drive it, and Set changes the files on disk.

use std::fs;
use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::handlers::process_popup_input;
#[cfg(unix)]
use funny_file_manager::models::Check;
use funny_file_manager::models::{App, AttributesField, FileName, Overlay, Panel};
use tempfile::TempDir;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

fn press(app: &mut App, code: KeyCode) {
    assert!(
        process_popup_input(app, key(code)),
        "the dialog consumes {code:?}"
    );
}

#[expect(
    clippy::panic,
    reason = "test assertion: a closed attributes dialog fails the calling test; clippy.toml \
              allows that in #[test] fns and #[cfg(test)] items, not in integration-test helpers"
)]
fn field(app: &App) -> AttributesField {
    match &app.overlay {
        Overlay::Attributes(dialog) => dialog.field,
        _ => panic!("the attributes dialog is open"),
    }
}

#[expect(
    clippy::expect_used,
    reason = "test fixture: a failed setup fails the calling test; clippy.toml allows that in \
              #[test] fns and #[cfg(test)] items, not in integration-test helpers"
)]
fn app_marking(dir: &Path, names: &[&str]) -> App {
    let mut app = App::new().expect("app");
    app.left_panel = Panel::new(dir.to_path_buf()).expect("panel");
    for name in names {
        app.left_panel.marked.insert(FileName::from(*name));
    }
    app
}

#[cfg(unix)]
#[expect(
    clippy::expect_used,
    reason = "test probe: an unreadable file fails the calling test; clippy.toml allows that in \
              #[test] fns and #[cfg(test)] items, not in integration-test helpers"
)]
fn mode(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).expect("metadata").permissions().mode() & 0o777
}

#[cfg(unix)]
#[expect(
    clippy::expect_used,
    reason = "test fixture: a failed setup fails the calling test; clippy.toml allows that in \
              #[test] fns and #[cfg(test)] items, not in integration-test helpers"
)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("chmod");
}

/// Two marked files with different modes: the disagreeing box opens as `[?]`,
/// one press each on two boxes, Enter — and each file keeps the bits that
/// were not touched.
#[cfg(unix)]
#[test]
fn ctrl_a_sets_one_bit_on_every_marked_file_and_keeps_the_rest() {
    let temp = TempDir::new().expect("temp");
    let a = temp.path().join("a.sh");
    let b = temp.path().join("b.sh");
    fs::write(&a, "x").expect("write");
    fs::write(&b, "x").expect("write");
    set_mode(&a, 0o644);
    set_mode(&b, 0o600);
    let mut app = app_marking(temp.path(), &["a.sh", "b.sh"]);

    funny_file_manager::ops::attributes::open(&mut app);
    let Overlay::Attributes(dialog) = &app.overlay else {
        panic!("Ctrl+A opens the dialog");
    };
    assert_eq!(dialog.headline(), "2 objects");
    assert_eq!(dialog.flags[3].current, Check::Mixed, "group read differs");

    // Owner execute is the third box; Right steps along the row.
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Right);
    assert_eq!(field(&app), AttributesField::Flag(2));
    press(&mut app, KeyCode::Char(' '));
    press(&mut app, KeyCode::Enter);

    assert!(!app.overlay.is_active());
    assert_eq!(mode(&a), 0o744);
    assert_eq!(mode(&b), 0o700);
    assert_eq!(app.left_panel.marked.len(), 0, "the marks were consumed");
    assert!(
        app.status_text().contains("2 object"),
        "{}",
        app.status_text()
    );
}

/// Esc leaves everything as it was, marks included.
#[cfg(unix)]
#[test]
fn escape_changes_nothing() {
    let temp = TempDir::new().expect("temp");
    let a = temp.path().join("a.txt");
    fs::write(&a, "x").expect("write");
    set_mode(&a, 0o644);
    let mut app = app_marking(temp.path(), &["a.txt"]);

    funny_file_manager::ops::attributes::open(&mut app);
    press(&mut app, KeyCode::Char(' '));
    press(&mut app, KeyCode::Esc);

    assert!(!app.overlay.is_active());
    assert_eq!(mode(&a), 0o644);
    assert_eq!(app.left_panel.marked.len(), 1);
}

/// A typed modification time lands on the file, to the second.
#[test]
fn a_typed_modification_time_is_written() {
    let temp = TempDir::new().expect("temp");
    let a = temp.path().join("dated.txt");
    fs::write(&a, "x").expect("write");
    let mut app = app_marking(temp.path(), &["dated.txt"]);

    funny_file_manager::ops::attributes::open(&mut app);
    while field(&app) != AttributesField::Modified {
        press(&mut app, KeyCode::Tab);
    }
    for _ in 0..25 {
        press(&mut app, KeyCode::Backspace);
    }
    for ch in "2010-10-10 10:10:10".chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    press(&mut app, KeyCode::Enter);

    assert!(!app.overlay.is_active());
    let modified = fs::metadata(&a)
        .expect("metadata")
        .modified()
        .expect("mtime");
    assert_eq!(
        funny_file_manager::models::format_time(modified),
        "2010-10-10 10:10:10"
    );
}
