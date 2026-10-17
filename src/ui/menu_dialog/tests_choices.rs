//! Unit tests: the choice list's colours and scrolling, and prompts and lists
//! keeping the cursor on screen as the terminal is resized.

use ratatui::style::Color;
use ratatui::{backend::TestBackend, Terminal};

use super::tests::screen_text;
use super::*;
use crate::{
    lineedit::LineEditor,
    models::{ChoiceTarget, MenuChoice, MenuPrompt},
};

#[test]
fn choices_use_white_on_teal_and_a_black_selected_row() {
    let dialog = MenuDialog::Choices {
        title: "Screens".to_owned(),
        rows: vec![MenuChoice {
            label: "Panels".to_owned(),
            target: ChoiceTarget::Close,
        }],
        selected: 0,
    };
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|f| render(f, &dialog, &mut DialogMouse::default()))
        .unwrap();
    let cells = &terminal.backend().buffer().content;
    let selected = cells.iter().find(|cell| cell.symbol() == "P").unwrap();
    assert_eq!(selected.fg, Color::Rgb(255, 255, 255));
    assert_eq!(selected.bg, Color::Rgb(0, 0, 0));
    assert!(cells
        .iter()
        .any(|cell| cell.symbol() == "╔" && cell.bg == Color::Rgb(0, 128, 128)));
}

#[test]
fn empty_and_scrolled_choices_render_and_small_prompts_do_not_panic() {
    let mut terminal = Terminal::new(TestBackend::new(60, 20)).unwrap();
    let mut dialog = MenuDialog::Choices {
        title: "Results".to_owned(),
        rows: vec![],
        selected: 0,
    };
    terminal
        .draw(|f| render(f, &dialog, &mut DialogMouse::default()))
        .unwrap();
    let text = screen_text(&terminal);
    assert!(text.contains("No entries"));
    if let MenuDialog::Choices { rows, selected, .. } = &mut dialog {
        *rows = (0..40)
            .map(|i| MenuChoice {
                label: format!("Result {i}"),
                target: ChoiceTarget::Close,
            })
            .collect();
        *selected = 39;
    }
    terminal
        .draw(|f| render(f, &dialog, &mut DialogMouse::default()))
        .unwrap();
    let text = screen_text(&terminal);
    assert!(text.contains("Result 39"));
}

#[test]
fn resizing_prompts_and_lists_keeps_the_cursor_on_screen() {
    let prompt = MenuDialog::Prompt {
        purpose: MenuPrompt::Filter,
        title: "Filter".to_owned(),
        caption: "目录".repeat(40),
        input: LineEditor::new(&"目录".repeat(40)),
    };
    let choices = MenuDialog::Choices {
        title: "Results".to_owned(),
        rows: vec![MenuChoice {
            label: "目录".repeat(100),
            target: ChoiceTarget::Close,
        }],
        selected: 0,
    };
    for width in [0, 1, 5, 20, 80] {
        for height in [0, 1, 5, 6, 7, 20] {
            assert_resize(&prompt, width, height);
            assert_resize(&choices, width, height);
        }
    }
}

fn assert_resize(modal: &MenuDialog, width: u16, height: u16) {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|f| render(f, modal, &mut DialogMouse::default()))
        .unwrap();
    if width > 0 && height > 0 {
        let cursor = terminal.get_cursor_position().unwrap();
        assert!(cursor.x < width && cursor.y < height);
    }
}
