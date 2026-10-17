//! Purpose: The task list's bottom row — the key bar, or the text bar that
//! takes its place while filtering, searching or typing a PID, with its
//! hints, caption and caret.

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::lineedit::LineEditor;
use crate::models::{Found, Modal, TaskList};

use super::super::super::fkey_bar::render_key_bar;
use super::super::super::{palette, printable_name, tail_cells, Cells};
use super::key_labels;

/// The bottom row: a text bar while one is open, the key bar otherwise.
pub(in crate::ui::tasks) fn render_bottom(f: &mut Frame<'_>, area: Rect, list: &TaskList) {
    let bar = match list.modal() {
        Modal::Filter(bar) => InputBar::new(FILTER_HINTS, "Filter", bar.line()),
        Modal::Search(bar) => InputBar::new(SEARCH_HINTS, "Search", bar.line()).found(bar.found()),
        Modal::PidJump(query) => InputBar::fixed(PID_HINTS, "PID", query.digits()),
        _ => {
            render_key_bar(f, area, &key_labels(list));
            return;
        }
    };
    bar.render(f, area);
}

/// A bar's keys and what each does.
type Hints = &'static [(&'static str, &'static str)];

const FILTER_HINTS: Hints = &[("Enter", "Done"), ("Esc", "Clear")];

const SEARCH_HINTS: Hints = &[("F3", "Next"), ("Enter", "Done"), ("Esc", "Cancel")];

const PID_HINTS: Hints = &[("Enter", "Done"), ("Esc", "Cancel")];

/// A text bar: its hints, its caption and what has been typed, split at the
/// caret.
struct InputBar<'a> {
    hints: Hints,
    caption: &'static str,
    before: &'a str,
    after: &'a str,
    text: Color,
}

impl<'a> InputBar<'a> {
    fn new(hints: Hints, caption: &'static str, line: &'a LineEditor) -> Self {
        Self {
            hints,
            caption,
            before: line.before_cursor(),
            after: line.after_cursor(),
            text: palette::DATA,
        }
    }

    /// A bar whose caret is always at the end of `typed`.
    const fn fixed(hints: Hints, caption: &'static str, typed: &'a str) -> Self {
        Self {
            hints,
            caption,
            before: typed,
            after: "",
            text: palette::DATA,
        }
    }

    /// Draws the query in the failed-prompt colour when nothing matched.
    const fn found(mut self, found: Found) -> Self {
        if matches!(found, Found::NoMatch) {
            self.text = palette::PROMPT_FAILED;
        }
        self
    }

    /// Draws the bar on `area` and puts the caret after what the caret
    /// follows, scrolling the text so the caret stays on the row.
    fn render(&self, f: &mut Frame<'_>, area: Rect) {
        let ground = Style::default()
            .fg(palette::DATA)
            .bg(palette::MENU_SELECTED_BG);
        let mut spans = hint_spans(self.hints);
        spans.push(Span::styled(format!("{}: ", self.caption), ground));
        let used: Cells = spans.iter().map(|span| Cells::of(&span.content)).sum();
        let room = Cells::columns(area.width).saturating_sub(used + Cells::new(1));
        let before = tail_cells(&printable_name(self.before), room);
        let caret = used + Cells::of(&before);
        let typed = format!("{before}{}", printable_name(self.after));
        spans.push(Span::styled(typed, ground.fg(self.text)));
        f.render_widget(Paragraph::new(Line::from(spans)).style(ground), area);
        let column = u16::try_from(caret.get()).unwrap_or(u16::MAX);
        if column < area.width && area.height > 0 {
            f.set_cursor_position((area.x.saturating_add(column), area.y));
        }
    }
}

/// ` Enter Done  Esc Cancel  `: each key in the key bar's number colours,
/// each action in its label colours.
fn hint_spans(hints: Hints) -> Vec<Span<'static>> {
    let key = Style::default()
        .fg(palette::DIRECTORY)
        .bg(palette::MENU_SELECTED_BG);
    let action = Style::default()
        .fg(palette::HIGHLIGHT_FG)
        .bg(palette::CURSOR_BG);
    let mut spans = vec![Span::styled(" ", key)];
    for (name, does) in hints {
        spans.push(Span::styled(format!("{name} "), key));
        spans.push(Span::styled(*does, action));
        spans.push(Span::styled("  ", key));
    }
    spans
}
