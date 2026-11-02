//! Purpose: Where the Language servers dialog draws: the framed window above
//! its key bar, the rows inside the frame — settings, list, detail pane,
//! message — and the rules between them, giving way on a short window.

use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, Paragraph},
    Frame,
};

use super::super::{far_block, palette, FAR_BLUE};
use super::ground;

/// Terminals shorter than this lose the detail pane; F3 still shows it.
const DETAIL_MIN_ROWS: u16 = 16;

/// Whether the list screen has room for the selected server's detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DetailPane {
    Shown,
    Hidden,
}

/// The window's rows inside its frame, top to bottom.
pub(super) struct Rows {
    pub(super) settings: Rect,
    pub(super) list: Rect,
    pub(super) detail: Option<Rect>,
    pub(super) message: Rect,
}

/// The framed window above the key bar, and the key bar.
pub(super) fn split(screen: Rect) -> Option<(Rect, Rect)> {
    if screen.height < 3 || screen.width < 4 {
        return None;
    }
    let window = Rect {
        height: screen.height - 1,
        ..screen
    };
    let bar = Rect {
        y: screen.y + screen.height - 1,
        height: 1,
        ..screen
    };
    Some((window, bar))
}

/// The window's frame, with `title` on top and `legend` in the bottom border.
pub(super) fn frame(title: &str, legend: &str) -> Block<'static> {
    far_block()
        .title(Line::from(format!(" {title} ")).centered())
        .title_bottom(Line::from(format!(" {legend} ")))
        .border_style(Style::default().fg(palette::FRAME))
        .style(ground())
}

/// Inside the frame, one blank column each side.
pub(super) fn inner(window: Rect) -> Rect {
    Rect {
        x: window.x + 2,
        y: window.y + 1,
        width: window.width.saturating_sub(4),
        height: window.height.saturating_sub(2),
    }
}

/// A `╟───╢` rule across the window at `y`, when `y` is above `below`, the
/// first row that belongs to something else.
pub(super) fn rule(f: &mut Frame<'_>, window: Rect, y: u16, below: u16) {
    if y >= below {
        return;
    }
    let line = format!(
        "╟{}╢",
        "─".repeat(usize::from(window.width.saturating_sub(2)))
    );
    f.render_widget(
        Paragraph::new(line).style(Style::default().fg(palette::FRAME).bg(FAR_BLUE)),
        Rect {
            y,
            height: 1,
            ..window
        },
    );
}

pub(super) fn line_at(f: &mut Frame<'_>, area: Rect, y: u16, line: Line<'static>) {
    if y < area.y + area.height {
        f.render_widget(
            Paragraph::new(line).style(ground()),
            Rect {
                y,
                height: 1,
                ..area
            },
        );
    }
}

/// The rows of the list screen for a window: the detail pane only on a
/// terminal tall enough, the list at most half of what is left. A window too
/// short for them all keeps the settings row, then the message row under
/// it; a row that does not fit has no height.
pub(super) fn rows(window: Rect, servers: usize, screen_rows: u16) -> Rows {
    let area = inner(window);
    let with_detail = screen_rows >= DETAIL_MIN_ROWS;
    let chrome = if with_detail { 5 } else { 4 };
    let body = area.height.saturating_sub(chrome);
    let wanted = u16::try_from(servers).unwrap_or(u16::MAX);
    let list_height = if with_detail {
        wanted.min(body / 2)
    } else {
        wanted.min(body)
    };
    let row = |y: u16, height: u16| Rect { y, height, ..area };
    let list = row(area.y + 2, list_height);
    let detail_y = list.y + list_height + 1;
    let detail = with_detail.then(|| row(detail_y, body - list_height));
    // Below two rows there is no message row, and no row for a rule either.
    let message = if area.height > 1 {
        row(area.y + area.height - 1, 1)
    } else {
        row(area.y, 0)
    };
    Rows {
        settings: row(area.y, area.height.min(1)),
        list,
        detail,
        message,
    }
}
