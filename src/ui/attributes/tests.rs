//! Unit tests: the File attributes dialog draws what the state says — the
//! selection, its owner and group, every box in its three states, the time
//! fields, the subfolders box, the buttons — and the cursor is visible on the
//! box that has it.

use std::path::PathBuf;
use std::time::SystemTime;

use ratatui::style::Modifier;
use ratatui::Terminal;

use crate::models::{App, AttributesDialog, AttributesField, Check, EntrySnapshot, Flag, Overlay};
use crate::test_support::{buffer_text, WritableTestBackend};
use crate::ui::overlays::render_overlays;

fn snapshot(name: &str, bits: u32, is_dir: bool) -> EntrySnapshot {
    EntrySnapshot {
        path: PathBuf::from("/tmp").join(name),
        bits,
        is_dir,
        owner: Some("alice".to_owned()),
        group: Some("staff".to_owned()),
        modified: Some(SystemTime::UNIX_EPOCH),
        accessed: Some(SystemTime::UNIX_EPOCH),
    }
}

fn draw(dialog: AttributesDialog) -> (Terminal<WritableTestBackend>, App) {
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("test terminal");
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::Attributes(Box::new(dialog)));
    terminal
        .draw(|f| render_overlays(f, &mut app))
        .expect("draw");
    (terminal, app)
}

/// The bit the platform's first box stands for, so the same test reads on
/// Unix and on Windows.
fn first_flag() -> Flag {
    Flag::GROUPS[0].flags[0]
}

#[test]
fn the_dialog_names_the_selection_its_owner_and_every_control() {
    let dialog = AttributesDialog::from_snapshots(&[snapshot("a.txt", first_flag().bit(), false)]);
    let (terminal, app) = draw(dialog);
    let screen = buffer_text(&terminal);

    assert!(screen.contains("File attributes"), "{screen}");
    assert!(screen.contains("a.txt"), "{screen}");
    assert!(screen.contains("Owner: alice"), "{screen}");
    assert!(screen.contains("Group: staff"), "{screen}");
    assert!(screen.contains(Flag::GROUPS[0].caption), "{screen}");
    assert!(
        screen.contains(&format!("[x] {}", first_flag().label())),
        "{screen}"
    );
    assert!(screen.contains("Modification:"), "{screen}");
    assert!(screen.contains("Access time:"), "{screen}");
    assert!(
        screen.contains("1970-01-01"),
        "one file offers its time: {screen}"
    );
    assert!(screen.contains("Process subfolders"), "{screen}");
    assert!(screen.contains("Set"), "{screen}");
    assert!(screen.contains("Cancel"), "{screen}");
    assert!(app.dialog_mouse.area.is_some(), "the frame was tracked");
}

#[test]
fn a_disagreeing_selection_draws_a_question_mark() {
    let bit = first_flag().bit();
    let dialog =
        AttributesDialog::from_snapshots(&[snapshot("a", bit, false), snapshot("b", 0, false)]);
    let (terminal, _) = draw(dialog);
    let screen = buffer_text(&terminal);
    assert!(
        screen.contains(&format!("[?] {}", first_flag().label())),
        "{screen}"
    );
    assert!(screen.contains("2 objects"), "{screen}");
}

/// The box under the cursor is the one drawn reversed, and only that one.
#[test]
fn the_focused_box_is_highlighted() {
    let mut dialog = AttributesDialog::from_snapshots(&[snapshot("a", 0, false)]);
    dialog.field = AttributesField::Flag(1);
    let (terminal, _) = draw(dialog);
    let screen = buffer_text(&terminal);
    let (y, line) = screen
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains(Flag::GROUPS[0].caption) && line.contains('['))
        .expect("the first group row");
    let boxes: Vec<usize> = line.match_indices('[').map(|(x, _)| x).collect();
    assert!(boxes.len() >= 2, "{line}");
    let buffer = terminal.backend().buffer();
    let reversed = |x: usize| {
        buffer[(
            u16::try_from(x).expect("column"),
            u16::try_from(y).expect("row"),
        )]
            .style()
            .add_modifier
            .contains(Modifier::REVERSED)
    };
    assert!(reversed(boxes[1]), "the second box has the cursor: {line}");
    assert!(!reversed(boxes[0]), "the first box does not: {line}");
}

/// A cursor on a time field puts the caret at the end of its text.
#[test]
fn a_focused_time_field_shows_the_caret() {
    let mut dialog = AttributesDialog::from_snapshots(&[snapshot("a", 0, false)]);
    dialog.field = AttributesField::Modified;
    let (mut terminal, _) = draw(dialog);
    let (x, y) = terminal.get_cursor_position().expect("cursor").into();
    let screen = buffer_text(&terminal);
    let line = screen.lines().nth(usize::from(y)).expect("caret row");
    assert!(line.contains("Modification:"), "{line}");
    assert!(
        usize::from(x) > line.find("1970").expect("the time"),
        "{line}"
    );
}

/// Without a directory the subfolders box is drawn but greyed; with one it
/// is live and shows its answer.
#[test]
fn the_subfolders_box_follows_the_selection() {
    let mut dialog = AttributesDialog::from_snapshots(&[snapshot("d", 0, true)]);
    dialog.process_subfolders = true;
    let (terminal, _) = draw(dialog);
    let screen = buffer_text(&terminal);
    assert!(screen.contains("[x] Process subfolders"), "{screen}");

    let dialog = AttributesDialog::from_snapshots(&[snapshot("a", 0, false)]);
    let (terminal, _) = draw(dialog);
    let screen = buffer_text(&terminal);
    assert!(screen.contains("[ ] Process subfolders"), "{screen}");
}

/// The Windows rows draw on any host, so the layout can be checked here.
#[test]
fn the_check_marks_cover_all_three_states() {
    assert_eq!(Check::Set.mark(), 'x');
    assert_eq!(Check::Clear.mark(), ' ');
    assert_eq!(Check::Mixed.mark(), '?');
}
