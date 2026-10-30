//! Purpose: Draws the task list (`Ctrl+W`) full screen, as the F3 viewer and
//! the F4 editor are: the meters, the process or network table, the info row,
//! the footer and the key bar, with whatever is open on the list drawn over
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
mod network;
mod placement;
mod table;

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

use crate::models::{Cursor, Table, TaskList, TaskTab};
use crate::mouse::DialogMouse;

use super::{far_block, palette, Cells, FAR_BLUE};
use placement::{layout, TaskLayout};
use table::RowBar;

pub use click::{task_click_at, TaskClick};

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
    let net_health = Some(list.net_diagnostics().overall_health());
    lines.push(meters::tasks_with_net(meters, net_health, width));
    f.render_widget(Paragraph::new(lines), layout.rows.meters);
}

fn render_table(f: &mut Frame<'_>, list: &TaskList, layout: &TaskLayout) {
    if layout.frame.is_empty() {
        return;
    }
    if list.tab() == TaskTab::Network {
        network::render(f, list, layout);
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
mod tests_chrome_bottom;
#[cfg(test)]
mod tests_click;
#[cfg(test)]
mod tests_dialog_lines;
#[cfg(test)]
mod tests_dialogs;
#[cfg(test)]
mod tests_dialogs_info;
#[cfg(test)]
mod tests_format;
#[cfg(test)]
mod tests_menus;
#[cfg(test)]
mod tests_meters;
#[cfg(test)]
mod tests_meters_lines;
#[cfg(test)]
mod tests_network_ui;
#[cfg(test)]
mod tests_screen;
#[cfg(test)]
mod tests_table;
#[cfg(test)]
mod tests_table_rows;
