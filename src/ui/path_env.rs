use super::{centered_rect, far_block, palette};
use crate::path_env::{Mode, PathEditor, Validation};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Clear, Paragraph},
    Frame,
};

#[cfg(test)]
mod tests;

pub(super) fn render(f: &mut Frame, editor: &PathEditor) {
    let area = centered_rect(94, 90, f.area());
    let capacity = usize::from(area.height.saturating_sub(9));
    let lines = rows(editor, capacity);
    let input_row = u16::try_from(lines.len().saturating_sub(2)).unwrap_or(u16::MAX);
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines)
            .block(far_block().title(format!(" PATH - {} ", editor.scope.label())))
            .style(Style::default().fg(palette::DRIVE_FG).bg(palette::MENU_BG)),
        area,
    );
    render_editor(f, editor, area, input_row);
}

fn rows(editor: &PathEditor, capacity: usize) -> Vec<Line<'_>> {
    let mut lines = vec![Line::from(editor.snapshot.location.clone())];
    let first = editor.selected.saturating_sub(capacity.saturating_sub(1));
    for (i, entry) in editor.entries.iter().enumerate().skip(first).take(capacity) {
        let text = format!(
            "{} [{:>16}] {}",
            if i == editor.selected { ">" } else { " " },
            entry.validation.label(),
            entry.text
        );
        let color = if entry.validation == Validation::Directory {
            palette::DRIVE_FG
        } else {
            Color::Red
        };
        lines.push(Line::styled(text, Style::default().fg(color)));
    }
    lines.extend(footer(editor));
    lines
}

fn render_editor(f: &mut Frame, editor: &PathEditor, area: Rect, row: u16) {
    let Mode::Edit { input, .. } = &editor.mode else {
        return;
    };
    if row >= area.height.saturating_sub(2) || area.width <= 13 {
        return;
    }
    let field = Rect::new(area.x + 12, area.y + 1 + row, area.width - 13, 1);
    super::menu_dialog::render_input(f, field, input);
}

fn footer(editor: &PathEditor) -> Vec<Line<'_>> {
    let mut lines = vec![Line::from(""), Line::from(editor.message.as_str())];
    let mode = match &editor.mode {
        Mode::Browse => {
            "Ins/F7 Add | F4 Edit | Del Remove | Ctrl+Up/Down Order | F5 Check | F2 Save | Esc Back"
        }
        Mode::Edit { .. } => {
            lines.push(Line::from("Directory:"));
            "Enter Accept and check | Esc Cancel"
        }
        Mode::ConfirmSave => {
            "Save PATH for this scope? Enter Save | Esc Cancel (system scope requires admin/root)"
        }
        Mode::ConfirmDiscard => {
            "Discard unsaved PATH changes? Enter Discard | Esc Continue editing"
        }
    };
    lines.push(Line::from(mode));
    lines
}
