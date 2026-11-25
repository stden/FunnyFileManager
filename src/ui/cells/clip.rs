//! Purpose: Text cut to a width in cells — at its end with a trailing
//! ellipsis, at its head for paths whose tail names the place, and a quoted
//! path that gives way before the message around it.

use std::borrow::Cow;

use super::super::panel::pad_cells;
use super::Cells;

/// Remaining horizontal scroll (`to_skip`) and visible width (`room`) across
/// styled spans on a single line.
#[derive(Clone, Copy)]
pub(in crate::ui) struct ColumnWindow {
    to_skip: Cells,
    room: Cells,
}

impl ColumnWindow {
    pub(in crate::ui) const fn new(skip: Cells, width: Cells) -> Self {
        Self {
            to_skip: skip,
            room: width,
        }
    }

    pub(in crate::ui) const fn is_full(self) -> bool {
        self.room.is_zero()
    }

    pub(in crate::ui) fn clip<'a>(&mut self, drawn: Cow<'a, str>) -> Option<Cow<'a, str>> {
        match drawn {
            Cow::Borrowed(s) => self.clip_str(s).map(Cow::Borrowed),
            Cow::Owned(s) => {
                let sub = self.clip_str(&s)?;
                if sub.len() == s.len() {
                    Some(Cow::Owned(s))
                } else {
                    Some(Cow::Owned(sub.to_owned()))
                }
            }
        }
    }

    /// Slice `src` into the horizontal window; if a wide glyph does not fit in
    /// the last remaining cell (`taken.len() < text.len()`), seal `room` to
    /// zero so later narrow spans cannot jump ahead of the cut glyph.
    fn clip_str<'a>(&mut self, src: &'a str) -> Option<&'a str> {
        let (text, dropped) = super::skip_cells(src, self.to_skip);
        self.to_skip = self.to_skip.saturating_sub(dropped);
        if text.is_empty() {
            return None;
        }
        let (taken, used) = take_cells_str(text, self.room);
        self.room = if taken.len() == text.len() {
            self.room.saturating_sub(used)
        } else {
            Cells::ZERO
        };
        (!taken.is_empty()).then_some(taken)
    }
}

/// Longest prefix of `text` that fits in `width` cells as a borrowed `&str`
/// slice, and the cells it uses.
pub(in crate::ui) fn take_cells_str(text: &str, width: Cells) -> (&str, Cells) {
    if super::is_printable_ascii(text) {
        let len = text.len().min(width.get());
        return (text.get(..len).unwrap_or(text), Cells::new(len));
    }
    let mut end = 0;
    let mut used = Cells::ZERO;
    for (idx, ch) in text.char_indices() {
        let ch_cells = Cells::of_char(ch);
        if used + ch_cells > width {
            return (text.get(..idx).unwrap_or_default(), used);
        }
        end = idx + ch.len_utf8();
        used = used + ch_cells;
    }
    (text.get(..end).unwrap_or(text), used)
}

/// First `width` cells of `text`, marked with a trailing ellipsis when cut.
pub(in crate::ui) fn head_cells(text: &str, width: Cells) -> String {
    if Cells::of(text) <= width {
        return text.to_owned();
    }
    if width.is_zero() {
        return String::new();
    }
    let (cut, _) = take_cells_str(text, width.saturating_sub(Cells(1)));
    let mut out = String::with_capacity(cut.len() + '…'.len_utf8());
    out.push_str(cut);
    out.push('…');
    out
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
    format!("…{}", last_cells(text, width.saturating_sub(Cells(1))))
}

/// `text` cut to `width` cells with `…` standing for its middle, so both its
/// start and its end stay: a path keeps its root and its file name, which are
/// what tell two of them apart.
pub(in crate::ui) fn middle_cells(text: &str, width: Cells) -> String {
    if Cells::of(text) <= width {
        return text.to_owned();
    }
    if width.is_zero() {
        return String::new();
    }
    let budget = width.saturating_sub(Cells(1)).get();
    let (head, _) = take_cells_str(text, Cells::new(budget.div_ceil(2)));
    format!("{head}…{}", last_cells(text, Cells::new(budget / 2)))
}

/// The last `width` cells of `text`. Walks backwards so the tail is what
/// survives, and drops a combining mark whose base character the cut removed.
fn last_cells(text: &str, width: Cells) -> String {
    let mut kept: Vec<char> = Vec::new();
    let mut used = Cells::ZERO;
    for ch in text.chars().rev() {
        let ch_cells = Cells::of_char(ch);
        if used + ch_cells > width {
            break;
        }
        kept.push(ch);
        used = used + ch_cells;
    }
    while kept.last().is_some_and(|&ch| Cells::of_char(ch).is_zero()) {
        kept.pop();
    }
    kept.into_iter().rev().collect()
}

/// A message quoting `path` between `before` and `after`, in `width` cells.
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
/// keeping at least its last component.
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
