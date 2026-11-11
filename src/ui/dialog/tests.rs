//! Purpose: Pin the shadow's colour scaling and dialog field cell padding/windowing.

use ratatui::style::Style;

use crate::lineedit::LineEditor;

use super::super::Cells;
use super::controls::{input_line, row_caret, FieldLook};
use super::row::Row;
use super::shadow::shadowed_component;

#[test]
fn a_shadow_keeps_three_eighths_of_each_colour_component() {
    assert_eq!(shadowed_component(0), 0);
    assert_eq!(shadowed_component(8), 3);
    assert_eq!(shadowed_component(9), 3, "integer maths rounds down");
    assert_eq!(shadowed_component(u8::MAX), 95);
}

#[test]
fn text_field_follows_its_focus_and_enabled_flags() {
    let row = Row::text_field("Name", "notes", true, true);
    assert!(row.is_focused());
    assert!(row.is_enabled());
    let idle = Row::text_field("Name", "notes", false, false);
    assert!(!idle.is_focused());
    assert!(!idle.is_enabled());
}

#[test]
fn input_line_pads_wide_cjk_characters_by_cells_not_chars() {
    let input = LineEditor::new("界界a");
    let row = Row::line(&input, true, true);
    let line = input_line(
        input.text(),
        FieldLook {
            focused: true,
            edit: row.edit(),
        },
        true,
        Style::default(),
        60,
    );
    let rendered: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(Cells::of(&rendered), Cells::new(60));
    assert!(rendered.ends_with('↓'));
}

#[test]
fn quality_title_marks_oversized_dialog_with_ellipsis() {
    use super::{Button, Ground};
    use ratatui::{backend::TestBackend, Terminal};

    let title = "Files highlighting and sort groups";
    let mut terminal = Terminal::new(TestBackend::new(40, 12)).unwrap();
    terminal
        .draw(|frame| {
            super::render_at(
                frame,
                title,
                Ground::Prompt,
                &[Row::Text("Readable")],
                &[Button::new("OK")],
                (0, 0),
            );
        })
        .unwrap();
    let screen: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(screen.contains("Files highlighting"), "{screen}");
    assert!(
        screen.contains('…'),
        "a clipped title must say it is truncated: {screen}"
    );
}

#[test]
fn long_input_windows_text_and_clamps_caret_before_drop_marker() {
    let long = "a".repeat(80);
    let input = LineEditor::new(&long);
    let row = Row::line(&input, true, true);
    let caret = row_caret(&row, 60).expect("focused input has caret");
    assert!(
        caret < 59,
        "caret must stay inside the 59-cell field before '↓', got {caret}"
    );

    let line = input_line(
        input.text(),
        FieldLook {
            focused: true,
            edit: row.edit(),
        },
        true,
        Style::default(),
        60,
    );
    let rendered: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(Cells::of(&rendered), Cells::new(60));
    assert!(rendered.ends_with('↓'));
}
