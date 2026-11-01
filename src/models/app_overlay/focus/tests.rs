//! Purpose: Verify that pasted text edits the open dialog's field and preserves other state.

use super::*;
use crate::models::{
    ApplyPrompt, CommandLine, DescribePrompt, FileName, InputPrompt, InputPurpose, MaskPrompt,
    MaskPurpose, NamePrompt, Panel,
};
use tempfile::TempDir;

mod forms;
mod remote_focus;
mod transfer;

fn app(root: &TempDir, overlay: Overlay) -> App {
    let panel = || Panel::from_entries(root.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel(), panel());
    app.set_overlay(overlay);
    app
}

fn paste(app: &mut App, text: &str) {
    app.focused_text_field()
        .expect("the dialog's text field is focused")
        .insert_str(text);
}

#[test]
fn pasting_into_the_command_line_preserves_its_history_suggestion() {
    let root = TempDir::new().unwrap();
    let state = CommandLine {
        ghost: Some("status".into()),
        ..CommandLine::from("git ")
    };
    let mut app = app(&root, Overlay::CommandLine(state));
    paste(&mut app, "diff");
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("the command line stays open");
    };
    assert_eq!(state.text(), "git diff");
    assert_eq!(state.ghost.as_deref(), Some("status"));
}

#[test]
fn pasting_a_name_or_mask_replaces_the_offered_selection() {
    let root = TempDir::new().unwrap();
    let mut app = app(
        &root,
        Overlay::NamePrompt(NamePrompt::rename(FileName::new("old.txt"))),
    );
    paste(&mut app, "new.txt");
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("the name prompt stays open");
    };
    assert_eq!(prompt.name(), "new.txt");
    assert_eq!(prompt.original(), "old.txt");
    app.set_overlay(Overlay::MaskPrompt(MaskPrompt::new(
        MaskPurpose::Select,
        "*.txt",
    )));
    paste(&mut app, "*.rs");
    let Overlay::MaskPrompt(prompt) = &app.overlay else {
        panic!("the mask prompt stays open");
    };
    assert_eq!(prompt.mask(), "*.rs");
    assert!(!prompt.is_offered());
}

#[test]
fn pasting_a_new_file_name_updates_the_input_in_place() {
    let root = TempDir::new().unwrap();
    let mut app = app(&root, Overlay::NewFileInput(LineEditor::new("notes")));
    paste(&mut app, ".txt");
    assert_eq!(app.get_new_file_input(), Some("notes.txt"));
}

#[test]
fn pasting_into_a_generic_prompt_keeps_its_purpose() {
    let root = TempDir::new().unwrap();
    let prompt = InputPrompt::new(InputPurpose::ImportNetBox, "sessions");
    let mut app = app(&root, Overlay::InputPrompt(Box::new(prompt)));
    paste(&mut app, ".netbox");
    let Overlay::InputPrompt(prompt) = &app.overlay else {
        panic!("the input prompt stays open");
    };
    assert_eq!(prompt.purpose(), &InputPurpose::ImportNetBox);
    assert_eq!(prompt.input().text(), "sessions.netbox");
}

#[test]
fn pasting_an_apply_command_or_description_preserves_the_dialog_data() {
    let root = TempDir::new().unwrap();
    let mut app = app(&root, Overlay::ApplyCommand(ApplyPrompt::default()));
    paste(&mut app, "echo !.!");
    let Overlay::ApplyCommand(prompt) = &app.overlay else {
        panic!("the apply prompt stays open");
    };
    assert_eq!(prompt.command(), "echo !.!");
    let prompt = DescribePrompt::new("first.txt".into(), "first", vec!["next.txt".into()]);
    app.set_overlay(Overlay::Describe(prompt));
    paste(&mut app, " description");
    let Overlay::Describe(prompt) = &app.overlay else {
        panic!("the describe prompt stays open");
    };
    assert_eq!(prompt.name(), "first.txt");
    assert_eq!(prompt.text(), "first description");
    assert_eq!(prompt.pending(), ["next.txt"]);
}

#[test]
fn non_text_overlays_offer_no_field_for_paste() {
    let root = TempDir::new().unwrap();
    let mut app = app(&root, Overlay::None);
    for overlay in [
        Overlay::None,
        Overlay::Help,
        Overlay::DeleteConfirm(Vec::new()),
    ] {
        app.set_overlay(overlay);
        assert!(app.focused_text_field().is_none());
    }
}
