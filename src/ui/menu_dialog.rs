//! Far-style forms and selection lists opened from the main menu.

use crate::{models::MenuDialog, mouse::DialogMouse};
use ratatui::{layout::Rect, style::Style, widgets::Paragraph, Frame};

use super::{dialog, palette, Cells};

pub(super) mod choices;
pub use choices::row_at;

pub(super) fn render(f: &mut Frame<'_>, modal: &MenuDialog, tracker: &mut DialogMouse) {
    match modal {
        MenuDialog::Prompt {
            title,
            caption,
            input,
            ..
        } => {
            render_prompt(f, tracker, title, caption, input);
        }
        MenuDialog::Choices {
            title,
            rows,
            selected,
        } => {
            choices::render(f, title, rows, *selected);
        }
    }
}

fn render_prompt(
    f: &mut Frame<'_>,
    tracker: &mut DialogMouse,
    title: &str,
    caption: &str,
    input: &crate::lineedit::LineEditor,
) {
    let width = Cells::columns(f.area().width.saturating_sub(8).min(66));
    let mut captions = caption_lines(caption, width);
    captions.truncate(usize::from(f.area().height.saturating_sub(7)));
    let mut rows: Vec<_> = captions
        .iter()
        .map(|line| dialog::Row::Label(line))
        .collect();
    rows.push(dialog::Row::input("", false, false));
    let buttons = [
        dialog::Button::new("OK").default_button(),
        dialog::Button::new("Cancel (F10)"),
    ];
    dialog::render_tracked(f, tracker, title, dialog::Ground::Prompt, &rows, &buttons);
    if let Some(body) = tracker.body {
        let offset = u16::try_from(captions.len()).unwrap_or(0);
        if offset < body.height.saturating_sub(2) {
            render_input(f, Rect::new(body.x, body.y + offset, body.width, 1), input);
        }
    }
}

fn caption_lines(caption: &str, width: Cells) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in caption.split('\n') {
        let mut remaining = paragraph;
        while Cells::of(remaining) > width {
            let (head, _) = super::take_cells(remaining, width);
            if head.is_empty() {
                break;
            }
            let split = head
                .rfind(' ')
                .filter(|index| *index > 0)
                .unwrap_or(head.len());
            let Some((line, rest)) = remaining.split_at_checked(split) else {
                break;
            };
            lines.push(line.to_owned());
            remaining = rest.trim_start();
        }
        lines.push(remaining.to_owned());
    }
    lines
}

pub(super) fn render_input(f: &mut Frame<'_>, area: Rect, input: &crate::lineedit::LineEditor) {
    let head = super::tail_cells(input.before_cursor(), Cells::columns(area.width - 1));
    let caret = u16::try_from(Cells::of(&head).get()).unwrap_or(0);
    let text = format!("{head}{}", input.after_cursor());
    f.render_widget(
        Paragraph::new(text).style(Style::default().bg(palette::FIELD_BG).fg(palette::FIELD_FG)),
        area,
    );
    f.set_cursor_position((area.x + caret, area.y));
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_choices;
