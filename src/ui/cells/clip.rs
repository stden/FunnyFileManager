//! Purpose: Text cut to a width in cells — at its end with a trailing
//! ellipsis, at its head for paths whose tail names the place, and a quoted
//! path that gives way before the message around it.

use super::super::panel::pad_cells;
use super::{take_cells, Cells};

/// First `width` cells of `text`, marked with a trailing ellipsis when cut.
pub(in crate::ui) fn head_cells(text: &str, width: Cells) -> String {
    if Cells::of(text) <= width {
        return text.to_owned();
    }
    if width.is_zero() {
        return String::new();
    }
    let (mut cut, _) = take_cells(text, width.saturating_sub(Cells(1)));
    cut.push('…');
    cut
}

/// `text` cut to `width` cells with a trailing ellipsis, then padded with
/// blanks to exactly `width` — [`head_cells`] and [`pad_cells`] composed.
///
/// One function rather than the pair at each call site: a column that cuts
/// without padding drifts out of alignment whenever a double-width glyph is
/// dropped whole and the cut comes up one cell short.
pub fn fit_name(text: &str, width: Cells) -> String {
    pad_cells(&head_cells(text, width), width)
}

/// Last `width` cells of `text`, marked with a leading ellipsis when cut.
/// Paths are cut at the head: the tail names the current directory.
pub(in crate::ui) fn tail_cells(text: &str, width: Cells) -> String {
    if Cells::of(text) <= width {
        return text.to_owned();
    }
    if width.is_zero() {
        return String::new();
    }
    // Walk backwards so the tail — the part that identifies the directory —
    // is what survives, keeping only as much as fits beside the ellipsis.
    let mut kept: Vec<char> = Vec::new();
    let mut used = Cells(1); // the leading '…'
    for ch in text.chars().rev() {
        let ch_cells = Cells::of_char(ch);
        if used + ch_cells > width {
            break;
        }
        kept.push(ch);
        used = used + ch_cells;
    }
    let mut cut = String::from("…");
    cut.extend(kept.into_iter().rev());
    cut
}

/// A message quoting `path` between `before` and `after`, in `width` cells.
///
/// The path gives way first, cut from its head as [`tail_cells`] cuts, since
/// what identifies the file is at its tail and the reason for the message is
/// in `after`. It keeps at least its last component — the file the message is
/// about — and only then is the line cut at its end.
pub(in crate::ui) fn fit_quoting_path(
    before: &str,
    path: &str,
    after: &str,
    width: Cells,
) -> String {
    let words = Cells::of(before) + Cells::of(after);
    head_cells(
        &format!("{before}{}{after}", fit_path(path, words, width)),
        width,
    )
}

/// `path` as a line of `width` cells quotes it beside `words` cells of other
/// text: whole when it fits, else cut from its head as [`tail_cells`] cuts,
/// keeping at least its last component. [`fit_quoting_path`]'s rule, for a
/// line whose parts are drawn in different styles.
pub(in crate::ui) fn fit_path(path: &str, words: Cells, width: Cells) -> String {
    let room = width.saturating_sub(words).max(last_component_cells(path));
    tail_cells(path, room)
}

/// The cells `path` keeps cut to its last component: the ellipsis, the
/// separator and the name — or the whole path, when that is shorter.
fn last_component_cells(path: &str) -> Cells {
    let name = std::path::Path::new(path)
        .file_name()
        .map_or(Cells::ZERO, |name| Cells::of(&name.to_string_lossy()));
    Cells::of(path).min(name + Cells(2))
}
