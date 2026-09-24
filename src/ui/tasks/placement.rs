//! Purpose: Places every part of the task list on a screen — frame, meter
//! rows, titles, table body, info row and bottom bar — from the header shape
//! paging uses, so drawing, paging and clicks agree on the same rows.

use std::ops::Range;

use ratatui::layout::Rect;

use crate::models::{Cursor, HeaderShape, TaskList, Viewport};
use crate::scroll::visible_window;

use super::super::{far_block, Cells};
use super::table::Columns;

/// Screens narrower than this show only the key bar.
const MIN_WIDTH: u16 = 10;

/// The rule under the meters.
const METER_RULE: u16 = 1;

/// Where each part of the task list goes on a screen.
pub(super) struct TaskLayout {
    /// The double frame: every row but the last.
    pub(super) frame: Rect,
    pub(super) shape: HeaderShape,
    pub(super) rows: FrameRows,
    /// The last row: the key bar or a text bar.
    pub(super) bottom: Rect,
    pub(super) columns: Columns,
    /// The shown rows the body draws, by their place in the view.
    pub(super) window: Range<usize>,
}

/// The rows inside the frame, top to bottom.
pub(super) struct FrameRows {
    /// The summary and core rows.
    pub(super) meters: Rect,
    /// The rule under the meters, across the frame; empty without meters.
    pub(super) meter_rule: Rect,
    pub(super) titles: Rect,
    /// The table rows.
    pub(super) body: Rect,
    /// The rule under the table, across the frame.
    pub(super) closing: Rect,
    pub(super) info: Rect,
}

/// Rows handed out top to bottom from an area, none past its end.
struct Rows {
    area: Rect,
    next: u16,
}

impl Rows {
    const fn new(area: Rect) -> Self {
        Self { area, next: 0 }
    }

    /// The next `count` rows, fewer where the area runs out.
    fn take(&mut self, count: u16) -> Rect {
        let height = count.min(self.area.height.saturating_sub(self.next));
        let taken = Rect {
            y: self.area.y.saturating_add(self.next),
            height,
            ..self.area
        };
        self.next = self.next.saturating_add(height);
        taken
    }

    fn left(&self) -> u16 {
        self.area.height.saturating_sub(self.next)
    }
}

/// Where `list` draws on `screen`. On a screen too short for the chrome
/// and one table row, the table rows go first; below [`MIN_WIDTH`] columns
/// only the bottom row is placed.
pub(super) fn layout(screen: Rect, list: &TaskList) -> TaskLayout {
    let bottom = Rect {
        y: screen.bottom().saturating_sub(1),
        height: screen.height.min(1),
        ..screen
    };
    let frame = if screen.width < MIN_WIDTH {
        Rect::default()
    } else {
        Rect {
            height: screen.height.saturating_sub(1),
            ..screen
        }
    };
    let shape = list.header_shape(Viewport::new(screen.height));
    let rows = frame_rows(frame, shape);
    let columns = Columns::for_list(list, Cells::columns(rows.titles.width));
    let cursor = list.cursor().map(Cursor::index);
    let shown = usize::from(rows.body.height);
    let window = visible_window(list.view().len(), shown, list.offset(), cursor);
    TaskLayout {
        frame,
        shape,
        rows,
        bottom,
        columns,
        window,
    }
}

/// The rows inside `frame` for a header of `shape`: the table gets its
/// page, or what is left of it on a screen too short for the chrome.
fn frame_rows(frame: Rect, shape: HeaderShape) -> FrameRows {
    let interior = far_block().inner(frame);
    let mut rows = Rows::new(interior);
    let meters = rows.take(shape.summary_rows().saturating_add(shape.cpu_rows()));
    let rule = if meters.height > 0 { METER_RULE } else { 0 };
    let meter_rule = across(frame, rows.take(rule));
    let titles = rows.take(1);
    let page = u16::try_from(shape.page()).unwrap_or(u16::MAX);
    let body = rows.take(page.min(rows.left().saturating_sub(2)));
    FrameRows {
        meters,
        meter_rule,
        titles,
        body,
        closing: across(frame, rows.take(1)),
        info: rows.take(1),
    }
}

/// `row` widened to the frame, for a rule that joins both borders.
const fn across(frame: Rect, row: Rect) -> Rect {
    Rect {
        x: frame.x,
        width: frame.width,
        ..row
    }
}
