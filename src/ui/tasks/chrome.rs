//! Purpose: The task list's chrome — the title and its freshness marker, the
//! footer, the info row, the key bar's labels and the text bars that take
//! its place while filtering, searching or typing a PID.
//!
//! Each text is one pure function of the list, so the tests pin every
//! variant without drawing.

mod bottom;
mod network;

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::models::{
    Arrangement, Cadence, CommandView, Health, Notice, Severity, TaskList, TaskTab, UserScope,
};

use super::super::fkey_bar::FKEY_COUNT;
use super::super::panel::clip_name;
use super::super::{palette, printable_name, Cells};
use super::format;
pub(super) use bottom::render_bottom;

/// The title without a marker.
const TITLE: &str = "Task list";
const NET_TITLE: &str = "Task list · Network";

/// The title: ` Task list ` (or ` Task list · Network `), with the most urgent
/// freshness marker.
pub(super) fn title(list: &TaskList) -> String {
    let base = match list.tab() {
        TaskTab::Processes => TITLE,
        TaskTab::Network => NET_TITLE,
    };
    let marker = match (list.health(), list.refresh().cadence()) {
        (Some(Health::Stalled), _) => " · not responding",
        (Some(Health::Failed(_)), _) => " · refresh failed",
        (Some(Health::Overdue), _) => " · stale",
        (_, Cadence::Held) => " · held",
        (Some(Health::Fresh) | None, Cadence::Live) => "",
    };
    format!(" {base}{marker} ")
}

/// The footer: how many processes or adapters are shown of how many, and what
/// narrows or marks them; `None` before the first table.
pub(super) fn footer(list: &TaskList) -> Option<String> {
    if list.tab() == TaskTab::Network {
        return network::footer(list);
    }
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
/// process (`pid name: command line`) or adapter, cut with Far's `}`.
pub(super) fn info(list: &TaskList, width: Cells) -> Line<'static> {
    if let Some(notice) = list.notice() {
        return notice_line(notice, width);
    }
    if list.tab() == TaskTab::Network {
        return network::info(list, width);
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
/// layout, `11` whether `F11` holds or resumes, and `12` switches tabs.
pub(super) fn key_labels(list: &TaskList) -> [(&'static str, &'static str); FKEY_COUNT] {
    let hold = match list.refresh().cadence() {
        Cadence::Live => "Hold",
        Cadence::Held => "Live",
    };
    if list.tab() == TaskTab::Network {
        return network::key_labels(list, hold);
    }
    let names = match list.command_view() {
        CommandView::CommandLine => "Name",
        CommandView::Name => "Cmd",
    };
    let tree = match list.options().arrangement {
        Arrangement::Flat => "Tree",
        Arrangement::Tree => "List",
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
        ("12", "Net"),
    ]
}
