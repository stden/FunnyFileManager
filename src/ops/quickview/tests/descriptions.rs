//! Folder descriptions use the configured filenames and keep the preview cache.

use super::{app_in, fixture, preview, refresh, select, toggle};
use crate::menu::SettingsPage;
use crate::models::{App, PanelKind, PanelSide, Preview};
use crate::scroll::Scrolled;
use std::fs;
use tempfile::TempDir;

fn folder_fixture(files: &[(&str, &[u8])]) -> (TempDir, App) {
    let dir = fixture();
    for (name, bytes) in files {
        fs::write(dir.path().join("nested").join(name), bytes).expect("description");
    }
    let mut app = app_in(&dir);
    select(&mut app, "nested");
    (dir, app)
}

fn assert_description(app: &App, lines: &[&str]) {
    let Preview::Text(text) = preview(app, PanelSide::Right) else {
        panic!("expected description: {:?}", preview(app, PanelSide::Right));
    };
    assert_eq!(text.lines, lines);
}

#[test]
fn readme_markdown_takes_priority_over_plain_text() {
    let (dir, mut app) = folder_fixture(&[
        ("README.txt", b"Plain notes"),
        ("README.md", b"# Folder notes\nDetails\n"),
    ]);
    toggle(&mut app);

    assert_description(&app, &["# Folder notes", "Details"]);
    let Preview::Text(text) = preview(&app, PanelSide::Right) else {
        panic!("expected description");
    };
    assert_eq!(text.path, dir.path().join("nested/README.md"));
    assert_eq!(app.active_panel, PanelSide::Left);
    assert_eq!(
        app.left_panel.selected_path(),
        Some(dir.path().join("nested"))
    );
    let PanelKind::QuickView(view) = &app.right_panel.kind else {
        panic!("expected quick view");
    };
    assert_eq!(view.shown, Some(dir.path().join("nested")));
}

#[test]
fn plain_text_readme_is_used_when_markdown_is_missing() {
    let (_dir, mut app) = folder_fixture(&[("README.txt", b"Plain notes")]);
    toggle(&mut app);
    assert_description(&app, &["Plain notes"]);
}

#[test]
fn configured_names_are_tried_in_order_ignoring_empty_entries() {
    let (_dir, mut app) = folder_fixture(&[
        ("README.md", b"Default notes"),
        ("NOTES.txt", b"Chosen notes"),
    ]);
    app.settings
        .set(
            SettingsPage::FolderDescriptions,
            " ; missing ; NOTES.txt ; README.md ; ",
        )
        .expect("settings");
    toggle(&mut app);
    assert_description(&app, &["Chosen notes"]);
}

#[test]
fn unsuitable_descriptions_are_skipped_before_a_readable_one() {
    let (dir, mut app) =
        folder_fixture(&[("binary", b"\0\x01"), ("README.txt", b"Readable notes")]);
    fs::create_dir(dir.path().join("nested/README.md")).expect("directory");
    fs::File::create(dir.path().join("nested/large"))
        .expect("file")
        .set_len(crate::utils::MAX_FILE_SIZE.bytes() + 1)
        .expect("sparse file");
    app.settings
        .set(
            SettingsPage::FolderDescriptions,
            "README.md;binary;large;README.txt",
        )
        .expect("settings");
    toggle(&mut app);
    assert_description(&app, &["Readable notes"]);
}

#[test]
fn unusable_readmes_fall_back_to_folder_statistics() {
    let (_dir, mut app) = folder_fixture(&[("README.md", b"\0\x01")]);
    toggle(&mut app);
    assert!(matches!(
        preview(&app, PanelSide::Right),
        Preview::Folder(_)
    ));
}

#[test]
fn information_setting_updates_the_open_folder_preview() {
    let (_dir, mut app) = folder_fixture(&[("README.md", b"Folder notes")]);
    app.settings
        .set(SettingsPage::Information, "off")
        .expect("settings");
    toggle(&mut app);
    assert!(matches!(
        preview(&app, PanelSide::Right),
        Preview::Folder(_)
    ));

    crate::ops::menu::settings::submit(&mut app, SettingsPage::Information, "on")
        .expect("enable descriptions");
    refresh(&mut app);
    assert_description(&app, &["Folder notes"]);

    crate::ops::menu::settings::submit(&mut app, SettingsPage::Information, "off")
        .expect("disable descriptions");
    refresh(&mut app);
    assert!(matches!(
        preview(&app, PanelSide::Right),
        Preview::Folder(_)
    ));
}

#[test]
fn changing_description_filenames_updates_the_open_preview() {
    let (_dir, mut app) = folder_fixture(&[
        ("README.md", b"Default notes"),
        ("NOTES.txt", b"Chosen notes"),
    ]);
    toggle(&mut app);
    crate::ops::menu::settings::submit(&mut app, SettingsPage::FolderDescriptions, "NOTES.txt")
        .expect("change description files");
    refresh(&mut app);
    assert_description(&app, &["Chosen notes"]);
}

#[test]
fn an_unchanged_folder_keeps_its_description_and_scroll() {
    let (dir, mut app) = folder_fixture(&[("README.md", b"First\nSecond\nThird\n")]);
    toggle(&mut app);
    if let PanelKind::QuickView(view) = &mut app.right_panel.kind {
        view.scroll_down(1, 1);
    }
    fs::write(dir.path().join("nested/README.md"), "Edited notes").expect("edit");
    refresh(&mut app);

    assert_description(&app, &["First", "Second", "Third"]);
    let PanelKind::QuickView(view) = &app.right_panel.kind else {
        panic!("expected quick view");
    };
    assert_eq!(view.scroll.lines(), 1);
}
