//! Purpose: clicks on a dialog's body rows: checkboxes answer, greyed rows and captions do not.

use super::*;

#[test]
fn clicking_a_copy_checkbox_row_ticks_it() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::CopyPrompt(Box::new(CopyPrompt::new(
        "a.txt",
        Path::new("/tmp"),
    ))));
    click_dialog_row(&mut app, 6);
    let Overlay::CopyPrompt(prompt) = &app.overlay else {
        panic!("copy dialog");
    };
    assert_eq!(prompt.field, CopyField::Timestamps);
    assert!(prompt.timestamps);
}

#[test]
fn clicking_a_greyed_copy_row_or_a_caption_does_nothing() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::CopyPrompt(Box::new(CopyPrompt::for_purpose(
        Purpose::Move,
        "a.txt",
        Path::new("/tmp"),
    ))));
    click_dialog_row(&mut app, 0);
    click_dialog_row(&mut app, 10);
    let Overlay::CopyPrompt(prompt) = &app.overlay else {
        panic!("copy dialog");
    };
    assert_eq!(prompt.field, CopyField::Destination);
    assert!(!prompt.filter);

    app.set_overlay(Overlay::Help);
    click_dialog_row(&mut app, 6);
    assert!(matches!(app.overlay, Overlay::Help));
}

// ─── clicking the rows of the Make folder dialog ────────────────────────────

/// A checkbox answers the click that reached it; every other control takes the
/// cursor and waits, which is what clicking a text field means everywhere.
#[test]
fn clicking_the_make_folder_rows_reaches_each_control() {
    use crate::models::NameField;

    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));

    click_dialog_row(&mut app, 3);
    assert_eq!(field(&app), NameField::Link, "the drop-down");

    click_dialog_row(&mut app, 5);
    assert_eq!(field(&app), NameField::Multiple);
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("still open");
    };
    assert!(prompt.multiple(), "the checkbox answered the click");

    click_dialog_row(&mut app, 1);
    assert_eq!(field(&app), NameField::Name);
}

/// A greyed control cannot be clicked into, and a row that is a caption or a
/// rule is not a control at all.
#[test]
fn clicking_a_grey_row_or_a_rule_changes_nothing() {
    use crate::models::NameField;

    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));

    click_dialog_row(&mut app, 4);
    assert_eq!(field(&app), NameField::Name, "Target is greyed");
    click_dialog_row(&mut app, 0);
    assert_eq!(field(&app), NameField::Name, "row 0 is the caption");
    click_dialog_row(&mut app, 99);
    assert_eq!(field(&app), NameField::Name, "and there is no row 99");
}

/// A body click in a dialog that has no controls in its body must do nothing
/// rather than something the user cannot predict.
#[test]
fn clicking_the_body_of_a_dialog_without_controls_does_nothing() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::QuitConfirm(Confirm::Yes));
    click_dialog_row(&mut app, 0);
    assert!(matches!(app.overlay, Overlay::QuitConfirm(Confirm::Yes)));
}

fn field(app: &App) -> crate::models::NameField {
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("the name dialog is open");
    };
    prompt.field()
}

use crate::models::{
    ArchiveField, ArchiveFormat, ArchiveMenu, ArchivePrompt, AttributesDialog, AttributesField,
    CleanupCandidate, CleanupField, CleanupKind, DiskCleanupDialog, EntrySnapshot, FindFile,
    FindFileField, SessionEdit, SessionField,
};

#[test]
fn clicking_rows_in_find_session_and_archive_dialogs() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);

    app.set_overlay(Overlay::FindFile(Box::<FindFile>::default()));
    click_dialog_row(&mut app, 3);
    if let Overlay::FindFile(d) = &app.overlay {
        assert_eq!(d.field(), FindFileField::Containing);
    }

    app.set_overlay(Overlay::SessionEdit(Box::new(SessionEdit::create())));
    click_dialog_row(&mut app, 2);
    if let Overlay::SessionEdit(d) = &app.overlay {
        assert_eq!(d.field(), SessionField::Host);
    }

    app.set_overlay(Overlay::ArchivePrompt(Box::new(ArchivePrompt::new(
        vec!["a.txt".into()],
        "a",
        dir.path(),
        vec![ArchiveFormat::TarGz],
    ))));
    click_dialog_row(&mut app, 5);
    if let Overlay::ArchivePrompt(p) = &app.overlay {
        assert_eq!((p.field, p.delete_after), (ArchiveField::DeleteAfter, true));
    }

    app.set_overlay(Overlay::ArchiveCommands(ArchiveMenu::default()));
    assert_eq!(
        click_dialog_row(&mut app, 2).map(|k| k.code),
        Some(KeyCode::Enter)
    );
}

#[test]
fn clicking_rows_in_cleanup_and_attributes_dialogs() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);

    let mut cleanup = DiskCleanupDialog::default();
    cleanup.candidates.clear();
    cleanup.candidates.push(CleanupCandidate {
        label: "Temp",
        path: dir.path().to_path_buf(),
        kind: CleanupKind::Contents,
        exists: true,
        checked: false,
        size: None,
    });
    app.set_overlay(Overlay::DiskCleanup(Box::new(cleanup)));
    click_dialog_row(&mut app, 0);
    if let Overlay::DiskCleanup(d) = &app.overlay {
        assert_eq!(
            (d.field, d.candidates[0].checked),
            (CleanupField::Item(0), true)
        );
    }

    let attrs = AttributesDialog::from_snapshots(&[EntrySnapshot {
        is_dir: true,
        ..EntrySnapshot::default()
    }]);
    app.set_overlay(Overlay::Attributes(Box::new(attrs)));
    click_dialog_row(&mut app, 3);
    if let Overlay::Attributes(d) = &app.overlay {
        assert_eq!(d.field, AttributesField::Flag(0));
    }
}
