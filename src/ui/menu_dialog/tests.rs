use super::*;
use crate::{
    lineedit::LineEditor,
    menu::MenuCommand,
    models::{ChoiceTarget, MenuChoice},
};
use ratatui::style::Color;
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn prompt_uses_a_compact_gray_form_with_real_buttons() {
    let dialog = MenuDialog::Prompt {
        command: MenuCommand::Filter,
        title: "Filter".to_owned(),
        caption: "File mask".to_owned(),
        input: LineEditor::new("*.rs"),
    };
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    let mut tracker = DialogMouse::default();
    terminal.draw(|f| render(f, &dialog, &mut tracker)).unwrap();
    let area = tracker.area.unwrap();
    assert!(area.width <= 50 && area.height <= 10);
    let buffer = terminal.backend().buffer();
    assert!(buffer
        .content
        .iter()
        .any(|cell| cell.bg == Color::Rgb(192, 192, 192)));
    assert!(screen_text(&terminal).contains("[ Cancel (F10) ]"));
    assert!(!screen_text(&terminal).contains("Enter: apply"));
}

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
fn prompt_wraps_the_caption_and_scrolls_to_the_input_cursor() {
    let mut terminal = Terminal::new(TestBackend::new(60, 20)).unwrap();
    let dialog = MenuDialog::Prompt {
        command: MenuCommand::ApplyCommand,
        title: "Apply command".to_owned(),
        caption: "Command to run for each selected file; {} is replaced with its quoted path. SSD/backups may retain data."
            .to_owned(),
        input: LineEditor::new(&"abcdef".repeat(20)),
    };
    terminal
        .draw(|f| render(f, &dialog, &mut DialogMouse::default()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    assert!(
        !buffer
            .content
            .iter()
            .any(|cell| cell.bg == palette::WARNING_BG),
        "a text prompt is a question, not a warning"
    );
    let text: String = buffer
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(text.contains("SSD/backups"));
    assert!(text.contains("Cancel (F10)"));
    assert!(text.contains("abcdef"));
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
fn small_prompts_do_not_panic() {
    let prompt = MenuDialog::Prompt {
        command: MenuCommand::Filter,
        title: String::new(),
        caption: String::new(),
        input: LineEditor::default(),
    };
    let mut tiny = Terminal::new(TestBackend::new(1, 1)).unwrap();
    tiny.draw(|f| render(f, &prompt, &mut DialogMouse::default()))
        .unwrap();
}

#[test]
fn caption_wrapping_preserves_unbroken_paths_and_unicode_boundaries() {
    for text in ["a".repeat(67), "目录".repeat(34)] {
        let lines = caption_lines(&text, Cells::new(66));
        assert_eq!(lines.concat(), text);
        assert!(lines.iter().all(|line| Cells::of(line) <= Cells::new(66)));
    }
}

#[test]
fn short_terminals_keep_input_clear_of_buttons() {
    let prompt = MenuDialog::Prompt {
        command: MenuCommand::Filter,
        title: "Filter".to_owned(),
        caption: "File mask".to_owned(),
        input: LineEditor::new("filename"),
    };
    let mut tracker = DialogMouse::default();
    let mut terminal = Terminal::new(TestBackend::new(60, 6)).unwrap();
    terminal.draw(|f| render(f, &prompt, &mut tracker)).unwrap();
    let body = tracker.body.unwrap();
    assert!(terminal
        .backend()
        .buffer()
        .content
        .iter()
        .all(|cell| cell.bg != palette::FIELD_BG));
    assert!(screen_text(&terminal).contains("Cancel (F10)"));
    assert_eq!(terminal.backend().buffer()[(body.x, body.y)].symbol(), "═");
}

#[test]
fn resizing_prompts_and_lists_keeps_the_cursor_on_screen() {
    let prompt = MenuDialog::Prompt {
        command: MenuCommand::Filter,
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

#[test]
fn editing_inside_a_long_unicode_input_keeps_text_after_the_cursor() {
    let mut input = LineEditor::new(&format!("{}end", "目录".repeat(40)));
    for _ in 0..3 {
        input.left();
    }
    let dialog = MenuDialog::Prompt {
        command: MenuCommand::Filter,
        title: "Filter".to_owned(),
        caption: "File mask".to_owned(),
        input,
    };
    let mut terminal = Terminal::new(TestBackend::new(60, 20)).unwrap();
    let mut tracker = DialogMouse::default();
    terminal.draw(|f| render(f, &dialog, &mut tracker)).unwrap();
    let cursor = terminal.get_cursor_position().unwrap();
    assert!(tracker.body.unwrap().contains(cursor));
    assert_eq!(terminal.backend().buffer()[cursor].symbol(), "e");
    assert_eq!(terminal.backend().buffer()[cursor].fg, palette::FIELD_FG);
}

fn screen_text(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}
