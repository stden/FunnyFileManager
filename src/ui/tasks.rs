//! Purpose: Draws the task list (`Ctrl+W`) full screen, as the F3 viewer and
//! the F4 editor are: the meters, the process table, the info row, the
//! footer and the key bar, with whatever is open on the list drawn over
//! them — and says what a click on that screen lands on ([`click`]).
//!
//! One [`layout`] places every part, from the header shape paging uses, so
//! the rows the renderer draws, the rows `PgDn` moves by and the rows a
//! click selects are the same rows. Rendering reads the model only.

mod chrome;
mod click;
mod dialogs;
mod format;
mod menus;
mod meters;
mod table;

use std::ops::Range;

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

use crate::models::{Cursor, HeaderShape, Table, TaskList, Viewport};
use crate::mouse::DialogMouse;
use crate::scroll::visible_window;

use super::{far_block, palette, Cells, FAR_BLUE};
use table::{Columns, RowBar};

pub use click::{task_click_at, TaskClick};

/// Screens narrower than this show only the key bar.
const MIN_WIDTH: u16 = 10;
/// The rule under the meters.
const METER_RULE: u16 = 1;

/// Where each part of the task list goes on a screen.
struct TaskLayout {
    /// The double frame: every row but the last.
    frame: Rect,
    shape: HeaderShape,
    rows: FrameRows,
    /// The last row: the key bar or a text bar.
    bottom: Rect,
    columns: Columns,
    /// The shown rows the body draws, by their place in the view.
    window: Range<usize>,
}

/// The rows inside the frame, top to bottom.
struct FrameRows {
    /// The summary and core rows.
    meters: Rect,
    /// The rule under the meters, across the frame; empty without meters.
    meter_rule: Rect,
    titles: Rect,
    /// The table rows.
    body: Rect,
    /// The rule under the table, across the frame.
    closing: Rect,
    info: Rect,
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
fn layout(screen: Rect, list: &TaskList) -> TaskLayout {
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

/// Draws the task list over the whole screen.
pub(super) fn render(f: &mut Frame<'_>, tracker: &mut DialogMouse, list: &TaskList) {
    let screen = f.area();
    let layout = layout(screen, list);
    f.render_widget(Clear, screen);
    render_frame(f, list, &layout);
    render_table(f, list, &layout);
    chrome::render_bottom(f, layout.bottom, list);
    dialogs::render(f, tracker, list, layout.rows.body);
}

fn render_frame(f: &mut Frame<'_>, list: &TaskList, layout: &TaskLayout) {
    if layout.frame.is_empty() {
        return;
    }
    let ground = Style::default().bg(FAR_BLUE);
    let edge = Style::default().fg(palette::FRAME).bg(FAR_BLUE);
    let title = Style::default().fg(palette::DATA).bg(FAR_BLUE);
    let mut block = far_block()
        .border_style(edge)
        .style(ground)
        .title(Line::from(Span::styled(chrome::title(list), title)).centered());
    if let Some(footer) = chrome::footer(list) {
        block = block.title_bottom(Line::from(Span::styled(footer, title)).centered());
    }
    f.render_widget(block, layout.frame);
    render_meters(f, list, layout);
    let info = chrome::info(list, Cells::columns(layout.rows.info.width));
    f.render_widget(Paragraph::new(info), layout.rows.info);
}

fn render_meters(f: &mut Frame<'_>, list: &TaskList, layout: &TaskLayout) {
    let Some(snapshot) = list.snapshot() else {
        return;
    };
    if layout.rows.meters.height == 0 {
        return;
    }
    let meters = snapshot.meters();
    let width = Cells::columns(layout.rows.meters.width);
    let mut lines = vec![meters::summary(meters, width)];
    if let Some(per_bar) = layout.shape.cores_per_bar() {
        lines.extend(meters::cores(
            meters,
            per_bar,
            layout.shape.cpu_rows(),
            width,
        ));
    }
    lines.push(meters::tasks(meters, width));
    f.render_widget(Paragraph::new(lines), layout.rows.meters);
}

fn render_table(f: &mut Frame<'_>, list: &TaskList, layout: &TaskLayout) {
    if layout.frame.is_empty() {
        return;
    }
    let edge = Style::default().fg(palette::FRAME).bg(FAR_BLUE);
    let rule = |left, junction, right| {
        Paragraph::new(layout.columns.rule(left, junction, right)).style(edge)
    };
    f.render_widget(rule('╟', '┬', '╢'), layout.rows.meter_rule);
    f.render_widget(rule('╟', '┴', '╢'), layout.rows.closing);
    let titles = layout.columns.titles(list);
    f.render_widget(Paragraph::new(titles), layout.rows.titles);
    f.render_widget(Paragraph::new(body_lines(list, layout)), layout.rows.body);
}

/// The body's rows: the shown processes in the window, the cursor's lit,
/// and the column rules below the last; or why there is no table.
fn body_lines(list: &TaskList, layout: &TaskLayout) -> Vec<Line<'static>> {
    let rows = usize::from(layout.rows.body.height);
    if let Table::Failed(error) = list.table() {
        let sentence = Span::styled(error.to_string(), Style::default().fg(palette::DATA));
        return vec![Line::from(sentence)];
    }
    let cursor = list.cursor().map(Cursor::index);
    let shown = list.view().get(layout.window.clone()).unwrap_or_default();
    let drawn = shown
        .iter()
        .zip(layout.window.clone())
        .map(|(&row, index)| {
            let bar = if cursor == Some(index) {
                RowBar::Cursor
            } else {
                RowBar::Plain
            };
            layout.columns.row(list, row, bar)
        });
    let blank = std::iter::repeat_with(|| layout.columns.empty());
    drawn.chain(blank).take(rows).collect()
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_chrome;
#[cfg(test)]
mod tests_click;
#[cfg(test)]
mod tests_dialog_lines;
#[cfg(test)]
mod tests_dialogs;
#[cfg(test)]
mod tests_format;
#[cfg(test)]
mod tests_menus;
#[cfg(test)]
mod tests_meters;
#[cfg(test)]
mod tests_screen;
#[cfg(test)]
mod tests_table;
