use super::*;
use crate::{lineedit::LineEditor, menu::MenuCommand};
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

pub(super) fn screen_text(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}
