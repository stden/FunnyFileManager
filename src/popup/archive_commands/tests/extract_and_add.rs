//! Purpose: the menu's Extract and Add rows — each opens its prompt on the right folder or archive, and Add needs marked files and an archive that can grow.

use super::*;

#[test]
fn extract_here_opens_the_extract_prompt_on_this_folder() {
    let home = TempDir::new().expect("home");
    let other = TempDir::new().expect("other");
    tar_archive(home.path(), "sample.tar", "cf");
    let mut app = app_open_on(&home, &other, "sample.tar");

    move_to(&mut app, ArchiveCommand::ExtractHere);
    press(&mut app, KeyCode::Enter);

    let Overlay::ExtractPrompt(prompt) = &app.overlay else {
        panic!("expected the extract prompt");
    };
    assert_eq!(prompt.destination, home.path().display().to_string());
}

#[test]
fn extract_to_other_panel_opens_the_extract_prompt_on_the_other_folder() {
    let home = TempDir::new().expect("home");
    let other = TempDir::new().expect("other");
    tar_archive(home.path(), "sample.tar", "cf");
    let mut app = app_open_on(&home, &other, "sample.tar");

    move_to(&mut app, ArchiveCommand::ExtractToOtherPanel);
    press(&mut app, KeyCode::Enter);

    let Overlay::ExtractPrompt(prompt) = &app.overlay else {
        panic!("expected the extract prompt");
    };
    assert_eq!(prompt.destination, other.path().display().to_string());
}

/// `extra.txt` marked beside the archive `name`, the cursor back on the
/// archive, and the menu open.
fn app_with_extra_marked(home: &TempDir, other: &TempDir, name: &str) -> App {
    std::fs::write(home.path().join("extra.txt"), "more\n").expect("write");
    let mut app = app_open_on(home, other, "extra.txt");
    app.close_overlay();
    app.get_active_panel().toggle_mark_selected();
    assert!(app.get_active_panel().select_named(name));
    crate::ops::archive::open_commands(&mut app);
    app
}

/// The marked files join the archive under the cursor: the dialog opens on
/// its path with the format fixed, and Enter appends rather than refusing
/// the archive for already existing.
#[test]
fn add_selected_opens_the_add_prompt_named_after_this_archive() {
    let home = TempDir::new().expect("home");
    let other = TempDir::new().expect("other");
    let archive = tar_archive(home.path(), "sample.tar", "cf");
    let mut app = app_with_extra_marked(&home, &other, "sample.tar");

    move_to(&mut app, ArchiveCommand::AddSelected);
    press(&mut app, KeyCode::Enter);

    let Overlay::ArchivePrompt(prompt) = &app.overlay else {
        panic!("expected the add prompt");
    };
    assert_eq!(prompt.name, archive.display().to_string());
    assert!(prompt.append);
    assert_eq!(prompt.sources, ["extra.txt"]);
    assert_eq!(prompt.offered, [crate::models::ArchiveFormat::Tar]);

    press(&mut app, KeyCode::Enter);
    assert!(!app.overlay.is_active());
    assert!(
        matches!(&app.status, Status::AddedToArchive { archive: a, kept: 0, .. } if *a == archive),
        "{:?}",
        app.status
    );
    let lines = list_archive(&archive).expect("listing");
    assert!(
        lines.iter().any(|line| line.contains("inside.txt")),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|line| line.contains("extra.txt")),
        "{lines:?}"
    );
}

/// With nothing marked there is nothing to add, and a compressed tar cannot
/// grow: both are said in the status bar instead of opening a dialog that
/// would fail.
#[test]
fn add_selected_needs_marked_files_and_an_archive_that_can_grow() {
    let home = TempDir::new().expect("home");
    let other = TempDir::new().expect("other");
    tar_archive(home.path(), "sample.tar", "cf");
    let mut app = app_open_on(&home, &other, "sample.tar");
    press(&mut app, KeyCode::Char('a'));
    assert!(!app.overlay.is_active());
    assert!(
        matches!(&app.status, Status::Error(text) if text.contains("mark")),
        "{:?}",
        app.status
    );

    tar_archive(home.path(), "packed.tar.gz", "czf");
    let mut app = app_with_extra_marked(&home, &other, "packed.tar.gz");
    press(&mut app, KeyCode::Char('a'));
    assert!(!app.overlay.is_active());
    assert!(
        matches!(&app.status, Status::Error(text) if text.contains("packed.tar.gz")),
        "{:?}",
        app.status
    );
}
