//! Unit tests: the Wipe dialog answers the same keys as F8's — Y and Enter
//! wipe, N, Esc and F10 cancel — and every other key is swallowed.

use std::fs;
use std::path::Path;

use crossterm::event::KeyCode;
use tempfile::TempDir;

use crate::models::{App, FileName, Overlay, Panel};
use crate::popup::process_popup_input;
use crate::popup::tests::key;

fn app_with_secret(dir: &Path) -> App {
    fs::write(dir.join("secret.txt"), b"payload").expect("write");
    let left = Panel::new(dir.to_path_buf()).expect("left panel");
    let right = Panel::new(dir.to_path_buf()).expect("right panel");
    let mut app = App::with_panels(left, right);
    assert!(app.get_active_panel().select_named("secret.txt"));
    crate::ops::wipe::open(&mut app);
    assert!(matches!(app.overlay, Overlay::WipeConfirm(_)));
    app
}

/// Opening sets the variant; Esc takes it down and runs nothing else.
#[test]
fn open_sets_the_variant_and_esc_closes_it() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_with_secret(dir.path());

    process_popup_input(&mut app, key(KeyCode::Char('x')));
    assert!(
        matches!(app.overlay, Overlay::WipeConfirm(_)),
        "other keys are consumed"
    );

    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(!app.overlay.is_active());
    assert!(
        dir.path().join("secret.txt").exists(),
        "nothing was touched"
    );
    assert_eq!(app.status.to_string(), "Wipe cancelled");
}

#[test]
fn y_and_enter_both_wipe() {
    for code in [KeyCode::Char('y'), KeyCode::Char('Y'), KeyCode::Enter] {
        let dir = TempDir::new().expect("temp dir");
        let mut app = app_with_secret(dir.path());

        assert!(process_popup_input(&mut app, key(code)));

        assert!(!app.overlay.is_active(), "{code:?}");
        assert!(!dir.path().join("secret.txt").exists(), "{code:?}");
        assert_eq!(app.status.to_string(), "Wiped 1 item(s)");
    }
}

#[test]
fn n_and_f10_cancel_like_esc() {
    for code in [KeyCode::Char('n'), KeyCode::Char('N'), KeyCode::F(10)] {
        let dir = TempDir::new().expect("temp dir");
        let mut app = app_with_secret(dir.path());

        assert!(process_popup_input(&mut app, key(code)));

        assert!(!app.overlay.is_active(), "{code:?}");
        assert!(dir.path().join("secret.txt").exists(), "{code:?}");
        assert_eq!(app.status.to_string(), "Wipe cancelled");
    }
}

#[test]
fn marked_entries_go_together_and_the_marks_with_them() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("a.txt"), b"a").expect("write");
    fs::write(dir.path().join("b.txt"), b"b").expect("write");
    let left = Panel::new(dir.path().to_path_buf()).expect("left panel");
    let right = Panel::new(dir.path().to_path_buf()).expect("right panel");
    let mut app = App::with_panels(left, right);
    app.get_active_panel()
        .marked
        .insert(FileName::from("a.txt"));
    app.get_active_panel()
        .marked
        .insert(FileName::from("b.txt"));
    crate::ops::wipe::open(&mut app);

    process_popup_input(&mut app, key(KeyCode::Enter));

    assert!(!dir.path().join("a.txt").exists());
    assert!(!dir.path().join("b.txt").exists());
    assert!(app.get_active_panel().marked.is_empty());
    assert_eq!(app.status.to_string(), "Wiped 2 item(s)");
}
