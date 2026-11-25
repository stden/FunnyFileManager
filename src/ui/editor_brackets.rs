//! Purpose: Highlight the matching delimiter at the editor caret.

use super::{screen_u16, Cells, EditorLine};
use crate::models::{
    pairs::{CachedPairs, PairCacheKey},
    Editor,
};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    Frame,
};

mod scan;
mod xml;

use scan::byte_at_char;
#[cfg(test)]
use scan::mate;

pub(super) const MAX_SCAN_LINES: usize = 1_000;

pub(super) fn render(frame: &mut Frame<'_>, editor: &Editor, area: Rect, top: usize, skip: Cells) {
    let pairs = resolve_pairs(editor);
    if let Some(spans) = pairs.xml_tags {
        xml::render_spans(frame, editor, area, top, skip, spans);
    }
    let Some(brackets) = pairs.brackets else {
        return;
    };
    for pos in brackets {
        if pos.line < top || pos.line >= top.saturating_add(usize::from(area.height)) {
            continue;
        }
        let Some(x) = column_on_screen(editor, pos.line, pos.column, skip) else {
            continue;
        };
        if x < usize::from(area.width) {
            frame.buffer_mut()[(
                area.x.saturating_add(screen_u16(x)),
                area.y.saturating_add(screen_u16(pos.line - top)),
            )]
                .set_style(
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::LightYellow)
                        .add_modifier(Modifier::BOLD),
                );
        }
    }
}

fn resolve_pairs(editor: &Editor) -> CachedPairs {
    let key = PairCacheKey {
        revision: editor.lsp.revision.0,
        cursor_line: editor.cursor_line,
        cursor_col: editor.cursor_col,
    };
    if let Some(cached) = editor.pair_cache.get().filter(|cached| cached.key == key) {
        return cached;
    }
    let resolved = CachedPairs {
        key,
        brackets: scan::matching_pair(editor),
        xml_tags: xml::compute_xml_pair(editor),
    };
    editor.pair_cache.set(Some(resolved));
    resolved
}

/// Screen column of `column_index` on `line_index` after `skip` cells; `None`
/// for a missing line or a character scrolled off the left edge.
pub(super) fn column_on_screen(
    editor: &Editor,
    line_index: usize,
    column_index: usize,
    skip: Cells,
) -> Option<usize> {
    let display = EditorLine {
        text: editor.lines.get(line_index)?,
        tab: editor.tab_size,
    };
    let cell = display.cells_before(crate::models::CharColumn::new(column_index));
    cell.get().checked_sub(display.dropped(skip).get())
}

#[cfg(test)]
mod tests;
