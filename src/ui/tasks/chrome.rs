//! Purpose: The task list's chrome — the title and its freshness marker, the
//! footer, the info row, the key bar's labels and the text bars that take
//! its place while filtering, searching or typing a PID.
//!
//! Each text is one pure function of the list, so the tests pin every
//! variant without drawing.

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::lineedit::LineEditor;
use crate::models::{
    Arrangement, Cadence, CommandView, Found, Health, Modal, Notice, Severity, TaskList, UserScope,
};

use super::super::fkey_bar::{render_key_bar, FKEY_COUNT};
use super::super::panel::clip_name;
use super::super::{palette, printable_name, tail_cells, Cells};
use super::format;

/// The title without a marker.
const TITLE: &str = "Task list";

/// The title: ` Task list `, with the most urgent freshness marker.
pub(super) fn title(list: &TaskList) -> String {
    let marker = match (list.health(), list.refresh().cadence()) {
        (Some(Health::Stalled), _) => " · not responding",
        (Some(Health::Failed(_)), _) => " · refresh failed",
        (Some(Health::Overdue), _) => " · stale",
        (_, Cadence::Held) => " · held",
        (Some(Health::Fresh) | None, Cadence::Live) => "",
    };
    format!(" {TITLE}{marker} ")
}

/// The footer: how many processes are shown of how many, and what narrows
/// or marks them; `None` before the first table.
pub(super) fn footer(list: &TaskList) -> Option<String> {
    let snapshot = list.snapshot()?;
    let total = list.rows().len();
    let shown = list.view().len();
    let mut parts = vec![if shown == total {
        processes(total)
    } else {
        format!("{shown} of {}", processes(total))
    }];
    let marked = list.marked_count();
    if marked > 0 {
        parts.push(format!("{marked} marked"));
    }
    if snapshot.unreadable() > 0 {
        parts.push(format!("{} unreadable", snapshot.unreadable()));
    }
    let filter = &list.options().filter;
    if !filter.is_empty() {
        parts.push(format!("filter {}", printable_name(filter.line())));
    }
    if let UserScope::Only(owner) = &list.options().scope {
        parts.push(format!("user {}", format::owner(owner)));
    }
    Some(format!(" {} ", parts.join(", ")))
}

/// `count` processes, in the singular for one.
fn processes(count: usize) -> String {
    match count {
        1 => "1 process".to_owned(),
        count => format!("{count} processes"),
    }
}

/// The info row: the notice until the next key, otherwise the cursor's
/// process as `pid name: command line`, cut with Far's `}`.
pub(super) fn info(list: &TaskList, width: Cells) -> Line<'static> {
    if let Some(notice) = list.notice() {
        return notice_line(notice, width);
    }
    let Some(process) = list.cursor().and_then(|cursor| list.row(cursor.key())) else {
        return Line::default();
    };
    let sample = process.sample();
    let text = format!(
        "{} {}: {}",
        sample.key.pid(),
        printable_name(&sample.name),
        printable_name(&sample.command)
    );
    Line::from(Span::styled(
        clip_name(&text, width),
        Style::default().fg(palette::FILE),
    ))
}

fn notice_line(notice: Notice, width: Cells) -> Line<'static> {
    let style = match notice.severity() {
        Severity::Information => Style::default()
            .fg(palette::DIALOG_FG)
            .bg(palette::DIALOG_BG),
        Severity::Warning => Style::default()
            .fg(palette::WARNING_FG)
            .bg(palette::WARNING_BG),
    };
    Line::from(Span::styled(clip_name(&notice.to_string(), width), style))
}

/// The key bar's labels: `2` names what `F2` switches to, `5` the other
/// layout, and `11` whether `F11` holds or resumes.
pub(super) fn key_labels(list: &TaskList) -> [(&'static str, &'static str); FKEY_COUNT] {
    let names = match list.command_view() {
        CommandView::CommandLine => "Name",
        CommandView::Name => "Cmd",
    };
    let tree = match list.options().arrangement {
        Arrangement::Flat => "Tree",
        Arrangement::Tree => "List",
    };
    let hold = match list.refresh().cadence() {
        Cadence::Live => "Hold",
        Cadence::Held => "Live",
    };
    [
        ("1", "Help"),
        ("2", names),
        ("3", "Info"),
        ("4", "Filter"),
        ("5", tree),
        ("6", "Sort"),
        ("7", "Nice"),
        ("8", "Kill"),
        ("9", "Signal"),
        ("10", "Quit"),
        ("11", hold),
        ("12", ""),
    ]
}

/// The bottom row: a text bar while one is open, the key bar otherwise.
pub(super) fn render_bottom(f: &mut Frame<'_>, area: Rect, list: &TaskList) {
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
