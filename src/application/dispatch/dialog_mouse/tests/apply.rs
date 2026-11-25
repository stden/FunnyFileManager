//! The Apply command dialog's OK and Cancel answer to the mouse like the
//! keyboard: a click names the button before Enter.

use std::fs;

use super::*;

fn open_with(app: &mut App, command: &str) {
    crate::ops::apply::open(app);
    for ch in command.chars() {
        popup::process_popup_input(app, KeyCode::Char(ch).into());
    }
}

fn on_cancel(app: &App) -> bool {
    let Overlay::ApplyCommand(prompt) = &app.overlay else {
        panic!("the apply dialog is open");
    };
    prompt.on_cancel()
}

#[test]
fn the_apply_dialog_buttons_name_the_button_they_land_on() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    crate::ops::apply::open(&mut app);

    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    assert!(!on_cancel(&app), "a click on OK stays on OK");
    assert_eq!(code(&mut app, 1), KeyCode::Enter);
    assert!(on_cancel(&app), "and a click on Cancel moves there");
    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    assert!(!on_cancel(&app), "a click on OK after Cancel goes back");
}

/// End to end: a click on OK runs the typed command, a click on Cancel runs
/// nothing — even when the keyboard focus was left on the other button.
#[test]
fn clicking_ok_runs_the_command_and_clicking_cancel_does_not() {
    let dir = TempDir::new().expect("temp");
    fs::write(dir.path().join("a.txt"), "alpha\n").expect("a.txt");
    let mut app = app_at(&dir);
    let mut term = terminal();
    assert!(app.get_active_panel().select_named("a.txt"));

    open_with(&mut app, "cp !.! !.bak");
    popup::process_popup_input(&mut app, KeyCode::Tab.into());
    assert!(on_cancel(&app), "the keyboard focus is on Cancel");
    click_text(&mut term, &mut app, "OK");
    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(
        fs::read_to_string(dir.path().join("a.bak")).expect("a.bak"),
        "alpha\n"
    );

    open_with(&mut app, "touch made-by-a-click");
    click_text(&mut term, &mut app, "Cancel");
    assert!(matches!(app.overlay, Overlay::None));
    assert!(!dir.path().join("made-by-a-click").exists());
}

#[test]
fn clicking_body_rows_focuses_simple_prompts() {
    use crate::models::{DescribePrompt, InputPrompt, InputPurpose, MaskPrompt, MaskPurpose};
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);

    let mut mask = MaskPrompt::new(MaskPurpose::Select, "*");
    mask.focus_cancel(true);
    app.set_overlay(Overlay::MaskPrompt(mask));
    click_dialog_row(&mut app, 1);
    assert!(matches!(&app.overlay, Overlay::MaskPrompt(p) if !p.on_cancel()));

    crate::ops::apply::open(&mut app);
    popup::process_popup_input(&mut app, KeyCode::Tab.into());
    click_dialog_row(&mut app, 1);
    assert!(!on_cancel(&app));

    let mut desc = DescribePrompt::new("a.txt".into(), "", Vec::new());
    desc.toggle_button();
    app.set_overlay(Overlay::Describe(desc));
    click_dialog_row(&mut app, 1);
    assert!(matches!(&app.overlay, Overlay::Describe(p) if !p.on_cancel()));

    let mut input = InputPrompt::new(InputPurpose::ImportNetBox, "");
    input.toggle_button();
    app.set_overlay(Overlay::InputPrompt(Box::new(input)));
    click_dialog_row(&mut app, 1);
    assert!(matches!(&app.overlay, Overlay::InputPrompt(p) if !p.on_cancel()));
}

#[test]
fn clicking_body_rows_focuses_extract_and_archive_fields() {
    use crate::models::{ArchiveField, ArchiveFormat, ArchivePrompt, ExtractField, ExtractPrompt};
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);

    let mut extract = ExtractPrompt::new(vec![dir.path().join("a.tar")], dir.path());
    extract.field = ExtractField::Cancel;
    app.set_overlay(Overlay::ExtractPrompt(Box::new(extract)));
    click_dialog_row(&mut app, 1);
    assert!(
        matches!(&app.overlay, Overlay::ExtractPrompt(p) if p.field == ExtractField::Destination)
    );

    app.set_overlay(Overlay::ArchivePrompt(Box::new(ArchivePrompt::new(
        vec!["a.txt".into()],
        "a",
        dir.path(),
        vec![ArchiveFormat::TarGz],
    ))));
    click_dialog_row(&mut app, 3);
    assert!(matches!(&app.overlay, Overlay::ArchivePrompt(p) if p.field == ArchiveField::Format));
    click_dialog_row(&mut app, 1);
    assert!(matches!(&app.overlay, Overlay::ArchivePrompt(p) if p.field == ArchiveField::Name));
    click_dialog_row(&mut app, 99);
}

#[test]
fn clicking_body_rows_focuses_attributes_and_find_file() {
    use crate::models::{
        AttributesDialog, AttributesField, EntrySnapshot, FindFile, FindFileField, Flag,
    };
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let attrs = AttributesDialog::from_snapshots(&[EntrySnapshot {
        is_dir: true,
        ..EntrySnapshot::default()
    }]);
    let after = 3 + Flag::GROUPS.len();
    app.set_overlay(Overlay::Attributes(Box::new(attrs)));
    click_dialog_row(&mut app, after + 1);
    assert!(matches!(&app.overlay, Overlay::Attributes(d) if d.field == AttributesField::Modified));
    click_dialog_row(&mut app, after + 2);
    assert!(matches!(&app.overlay, Overlay::Attributes(d) if d.field == AttributesField::Accessed));
    click_dialog_row(&mut app, after + 4);
    assert!(
        matches!(&app.overlay, Overlay::Attributes(d) if d.field == AttributesField::Subfolders)
    );

    let mut find = FindFile::default();
    find.tab(true);
    app.set_overlay(Overlay::FindFile(Box::new(find)));
    click_dialog_row(&mut app, 99);
    click_dialog_row(&mut app, 1);
    assert!(matches!(&app.overlay, Overlay::FindFile(d) if d.field() == FindFileField::Mask));
}

#[test]
fn clicking_body_rows_focuses_session_edit_and_cleanup() {
    use crate::models::{
        CandidateState, CleanupCandidate, CleanupKind, DiskCleanupDialog, SessionEdit,
    };
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::SessionEdit(Box::new(SessionEdit::create())));
    click_dialog_row(&mut app, 99);
    click_dialog_row(&mut app, 6);

    let cleanup = DiskCleanupDialog {
        candidates: vec![CleanupCandidate {
            label: "Missing",
            path: dir.path().to_path_buf(),
            kind: CleanupKind::Contents,
            state: CandidateState::Absent,
        }],
        ..Default::default()
    };
    app.set_overlay(Overlay::DiskCleanup(Box::new(cleanup)));
    click_dialog_row(&mut app, 99);
    click_dialog_row(&mut app, 0);
}
