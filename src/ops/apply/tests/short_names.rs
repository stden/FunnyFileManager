//! Purpose: Windows short names: resolving them and restoring a long name.

use super::*;

#[test]
fn native_short_names_resolve_to_the_same_file() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("long filename with spaces.txt");
    fs::write(&path, "contents").expect("file");
    let short = context::short_path(&path);
    assert_eq!(fs::read_to_string(short).expect("short path"), "contents");
    let missing = dir.path().join("nonexistent file.txt");
    assert_eq!(context::short_path(&missing), missing);
}

#[test]
fn preserve_short_name_restores_a_recreated_long_name() {
    let dir = TempDir::new().expect("tempdir");
    let names = crate::models::apply::Names {
        long: "original long name.txt".into(),
        short: "ORIG~1.TXT".into(),
    };
    fs::write(dir.path().join(&names.short), "recreated").expect("short file");
    context::restore_long_name(dir.path(), &names).expect("restore");
    assert_eq!(
        fs::read_to_string(dir.path().join(&names.long)).expect("long name"),
        "recreated"
    );
    assert!(!dir.path().join(&names.short).exists());
}

#[test]
fn preserve_short_name_never_replaces_an_existing_long_name() {
    let dir = TempDir::new().expect("tempdir");
    let names = crate::models::apply::Names {
        long: "existing.txt".into(),
        short: "OTHER.TXT".into(),
    };
    fs::write(dir.path().join(&names.short), "other").expect("short file");
    fs::write(dir.path().join(&names.long), "original").expect("long file");
    context::restore_long_name(dir.path(), &names).expect("preserve");
    assert_eq!(
        fs::read_to_string(dir.path().join(&names.long)).expect("long name"),
        "original"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join(&names.short)).expect("short name"),
        "other"
    );
}

#[test]
fn preserve_name_records_error_when_restore_fails_and_select_supports_right_panel() {
    let dir = TempDir::new().expect("tempdir");
    let mut app = app_at(&dir);
    app.active_panel = crate::models::PanelSide::Right;
    app.right_panel = Panel::new(dir.path().to_path_buf()).expect("right");
    let mut ctx = context::capture(&app, "a.txt");
    context::select(&mut ctx, &app, "a.txt");

    fs::write(dir.path().join("S.TXT"), "short").expect("short");
    fs::write(dir.path().join("blocker"), "file").expect("blocker");
    ctx.right.entry = crate::models::apply::Names {
        long: "blocker/child.txt".into(),
        short: "S.TXT".into(),
    };
    let ok = super::super::run_entry(&mut app, "!+!", &ctx, dir.path(), dir.path()).expect("run");
    assert!(!ok);
}
