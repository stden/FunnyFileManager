//! Unit tests: `Ctrl+Q` turns the passive panel into a preview of the active
//! cursor, and the preview follows that cursor.

use std::fs;

use tempfile::TempDir;

use super::{refresh, toggle};
use crate::models::{App, Panel, PanelKind, PanelSide, Preview, Refusal};

mod descriptions;

/// `alpha.txt`, `beta.txt`, a `nested` folder and a binary, in one directory.
fn fixture() -> TempDir {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("alpha.txt"), "alpha one\nalpha two\n").expect("write");
    fs::write(dir.path().join("beta.rs"), "fn beta() {}\n").expect("write");
    fs::write(dir.path().join("blob.bin"), b"\x00\x01\x02").expect("write");
    let nested = dir.path().join("nested");
    fs::create_dir(&nested).expect("mkdir");
    fs::write(nested.join("inner.txt"), "12345").expect("write");
    fs::write(nested.join("other.txt"), "1234567").expect("write");
    dir
}

fn app_in(dir: &TempDir) -> App {
    let panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    App::with_panels(panel.clone(), panel)
}

fn select(app: &mut App, name: &str) {
    let panel = app.get_active_panel();
    let index = panel
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("no entry {name}"));
    panel.select(Some(index));
}

fn preview(app: &App, side: PanelSide) -> &Preview {
    match &app.panel(side).kind {
        PanelKind::QuickView(view) => &view.preview,
        other => panic!("not a quick view: {other:?}"),
    }
}

#[test]
fn ctrl_q_previews_the_active_cursor_in_the_passive_panel() {
    let dir = fixture();
    let mut app = app_in(&dir);
    select(&mut app, "alpha.txt");

    toggle(&mut app);

    assert_eq!(
        app.active_panel,
        PanelSide::Left,
        "focus stays on the files"
    );
    assert_eq!(app.left_panel.kind, PanelKind::Files);
    let Preview::Text(text) = preview(&app, PanelSide::Right) else {
        panic!("expected text: {:?}", preview(&app, PanelSide::Right));
    };
    assert_eq!(text.lines, vec!["alpha one", "alpha two"]);
    assert_eq!(app.status.to_string(), "Quick view on");
}

#[test]
fn ctrl_q_again_brings_the_file_list_back() {
    let dir = fixture();
    let mut app = app_in(&dir);
    toggle(&mut app);
    toggle(&mut app);
    assert_eq!(app.right_panel.kind, PanelKind::Files);
    assert_eq!(app.status.to_string(), "Quick view off");
}

#[test]
fn ctrl_q_inside_the_preview_turns_that_panel_back_into_files() {
    let dir = fixture();
    let mut app = app_in(&dir);
    toggle(&mut app);
    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Right);

    toggle(&mut app);

    assert_eq!(app.right_panel.kind, PanelKind::Files);
    assert_eq!(app.left_panel.kind, PanelKind::Files);
}

#[test]
fn the_preview_follows_the_cursor_after_a_refresh() {
    let dir = fixture();
    let mut app = app_in(&dir);
    select(&mut app, "alpha.txt");
    toggle(&mut app);

    select(&mut app, "beta.rs");
    refresh(&mut app);

    let Preview::Text(text) = preview(&app, PanelSide::Right) else {
        panic!("expected text");
    };
    assert_eq!(text.lines, vec!["fn beta() {}"]);
    let PanelKind::QuickView(view) = &app.right_panel.kind else {
        panic!("not a quick view");
    };
    assert_eq!(
        view.shown.as_deref(),
        Some(dir.path().join("beta.rs").as_path())
    );
}

#[test]
fn a_refresh_on_the_same_file_keeps_the_scroll() {
    let dir = fixture();
    let mut app = app_in(&dir);
    select(&mut app, "alpha.txt");
    toggle(&mut app);
    if let PanelKind::QuickView(view) = &mut app.right_panel.kind {
        view.scroll_down(1, 1);
    }

    refresh(&mut app);

    let PanelKind::QuickView(view) = &app.right_panel.kind else {
        panic!("not a quick view");
    };
    assert_eq!(view.scroll.lines(), 1);
}

#[test]
fn a_folder_shows_its_totals() {
    let dir = fixture();
    let mut app = app_in(&dir);
    select(&mut app, "nested");
    toggle(&mut app);

    let Preview::Folder(stats) = preview(&app, PanelSide::Right) else {
        panic!("expected folder totals");
    };
    assert_eq!(stats.files, 2);
    assert_eq!(stats.directories, 0);
    assert_eq!(stats.total.bytes(), 12);
}

#[test]
fn a_binary_file_and_the_parent_row_are_declined_gracefully() {
    let dir = fixture();
    let mut app = app_in(&dir);
    select(&mut app, "blob.bin");
    toggle(&mut app);
    assert_eq!(
        preview(&app, PanelSide::Right),
        &Preview::Refused(Refusal::Binary)
    );

    select(&mut app, "..");
    refresh(&mut app);
    assert_eq!(preview(&app, PanelSide::Right), &Preview::Nothing);
}

#[test]
fn a_file_over_the_limit_is_not_read() {
    let dir = fixture();
    let huge = dir.path().join("huge.log");
    let file = fs::File::create(&huge).expect("create");
    file.set_len(crate::utils::MAX_FILE_SIZE.bytes() + 1)
        .expect("sparse file");
    let mut app = app_in(&dir);
    select(&mut app, "huge.log");

    toggle(&mut app);

    assert!(
        matches!(
            preview(&app, PanelSide::Right),
            Preview::Refused(Refusal::TooLarge(_))
        ),
        "{:?}",
        preview(&app, PanelSide::Right)
    );
}

#[test]
fn a_file_that_vanished_is_reported_not_crashed() {
    let dir = fixture();
    let mut app = app_in(&dir);
    select(&mut app, "alpha.txt");
    fs::remove_file(dir.path().join("alpha.txt")).expect("remove");

    toggle(&mut app);

    assert!(matches!(
        preview(&app, PanelSide::Right),
        Preview::Refused(Refusal::Unreadable(_))
    ));
}

#[test]
fn a_preview_opposite_a_tree_follows_the_tree_cursor() {
    let dir = fixture();
    let mut app = app_in(&dir);
    toggle(&mut app);
    app.left_panel.kind = PanelKind::Tree(Box::new(crate::models::FolderTree::rooted_at(
        dir.path().join("nested"),
    )));

    refresh(&mut app);

    let Preview::Folder(stats) = preview(&app, PanelSide::Right) else {
        panic!("expected folder totals");
    };
    assert_eq!(stats.files, 2);
}
