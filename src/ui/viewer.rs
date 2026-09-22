//! Far's F3 viewer: a status line on row 0, the file on every row below.
//!
//! Full screen and borderless, as the editor is. The old centred box with a
//! title cost the file ten columns and two rows to draw a frame around what
//! is, in Far, the whole screen — and the frame's title said less than the
//! status line does.

use std::path::Path;

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::code::highlighted_window;
use super::{fit_name, head_cells, palette, Cells, FAR_BLUE};
use crate::highlight::Highlighter;
use crate::models::{ViewSource, Viewer};

/// Rows the viewer spends on chrome: its status line, and nothing else.
const VIEWER_BAR_HEIGHT: u16 = 1;

/// Far's view mode letter. Far has `t`ext, `h`ex and `d`ump; this viewer
/// refuses binaries before it opens, so it only ever has the first.
const TEXT_MODE: &str = "t";

pub(super) fn render_viewer(f: &mut Frame, viewer: &Viewer) {
    let area = f.area();
    let body = Rect {
        y: area.y.saturating_add(VIEWER_BAR_HEIGHT),
        height: area.height.saturating_sub(VIEWER_BAR_HEIGHT),
        ..area
    };
    // Keys page from `App.viewport`, recorded at dispatch time. Drawing uses
    // this frame's height so a test that only renders still fills the body,
    // and a resize is visible on the next frame without the renderer writing
    // back into the model.
    let page = usize::from(body.height);

    f.render_widget(Clear, area);
    render_viewer_bar(
        f,
        viewer,
        Rect {
            height: VIEWER_BAR_HEIGHT.min(area.height),
            ..area
        },
        page,
    );

    render_viewer_body(f, viewer, body);
}

fn render_viewer_body(f: &mut Frame, viewer: &Viewer, body: Rect) {
    let page = usize::from(body.height);
    // A report is not a file: nothing to colour it as, and a title that
    // happens to look like a file name must not pick a language.
    let highlighter = match &viewer.source {
        ViewSource::File(file) => Highlighter::for_file(&file.path, &viewer.lines),
        ViewSource::Report(_) => Highlighter::new(Path::new("")),
    };
    let lines = highlighted_window(
        highlighter,
        &viewer.lines,
        viewer.scroll.lines(),
        page,
        Cells::new(viewer.col.cells()),
        Cells::columns(body.width),
    );
    f.render_widget(
        Paragraph::new(lines).style(Style::default().fg(palette::CODE_NORMAL).bg(FAR_BLUE)),
        body,
    );
}

/// The status line: the path on the left, Far's fields flush right.
fn render_viewer_bar(f: &mut Frame, viewer: &Viewer, area: Rect, page: usize) {
    let row = bar_row(
        viewer,
        Cells::columns(area.width),
        &crate::clock::now(),
        page,
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::raw(row))).style(
            Style::default()
                .bg(palette::VIEWER_BAR_BG)
                .fg(palette::VIEWER_BAR_FG),
        ),
        area,
    );
}

/// 100 once the last line is on screen, else how far down the top row is.
///
/// The first test also covers an empty `lines`, which would otherwise divide
/// by zero.
const fn percent(top: usize, rows: usize, total: usize) -> usize {
    if top.saturating_add(rows) >= total {
        100
    } else {
        top * 100 / total
    }
}

/// The status line's text, exactly `width` cells of it.
///
/// A pure function of the viewer and the clock so the tests can pin the row
/// to the character; `crate::clock::now()` would drift under them.
///
/// The path gives way first, cut with an ellipsis as the editor bar cuts its
/// name; when even the fields do not fit, the row is cut as a whole rather
/// than wrapped.
fn bar_row(viewer: &Viewer, width: Cells, clock: &str, page: usize) -> String {
    let (left, right) = match &viewer.source {
        ViewSource::File(file) => (
            file.path.display().to_string(),
            format!(
                "│{TEXT_MODE}│{:<5}│{:<8}│Col {:<4}│{:<5}│{clock}",
                file.encoding.label(),
                file.size.bytes(),
                viewer.col.cells(),
                format!(
                    "{}%",
                    percent(viewer.scroll.lines(), page, viewer.lines.len())
                ),
            ),
        ),
        ViewSource::Report(title) => (title.clone(), format!("│{clock}")),
    };
    let left = head_cells(&left, width.saturating_sub(Cells::of(&right)));
    let gap = " ".repeat(
        width
            .saturating_sub(Cells::of(&left) + Cells::of(&right))
            .get(),
    );
    fit_name(&format!("{left}{gap}{right}"), width)
}

#[cfg(test)]
mod tests;
