//! Syntax-highlighted text on the blue ground, shared by the editor and the
//! viewer.
//!
//! Its own module because both windows draw a file the same way — the same
//! colours per token, the same sideways clip, the same rule about where
//! highlighting starts — and the viewer importing them from `overlays`, which
//! in turn delegates to the viewer, would be a cycle of intent.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use std::borrow::Cow;

use super::palette;
use super::{expand_tabs, skip_cells, take_cells, Cells};
use crate::highlight::{cache::HighlightCache, Token};
use crate::models::TabSize;

pub(super) fn editor_window(
    editor: &crate::models::Editor,
    top: usize,
    rows: usize,
    skip: Cells,
    width: Cells,
) -> Vec<Line<'static>> {
    let mut cache = editor.highlights.borrow_mut();
    cache.prepare(&editor.path, &editor.lines, top.saturating_add(rows));
    let limit = skip + width;
    editor
        .lines
        .iter()
        .enumerate()
        .skip(top)
        .take(rows)
        .map(|(index, line)| {
            clip_runs(
                drawn_runs(cache.runs(index, line), editor.tab_size, limit),
                skip,
                width,
            )
        })
        .collect()
}

/// `runs` as the editor draws them up to `limit` cells: every TAB widened to
/// the next stop, the stops counted from the start of the line across the runs.
fn drawn_runs(runs: Vec<(&str, Token)>, tab: TabSize, limit: Cells) -> Vec<(Cow<'_, str>, Token)> {
    let mut at = Cells::ZERO;
    let mut out = Vec::with_capacity(runs.len());
    for (text, token) in runs {
        if at >= limit {
            break;
        }
        let drawn = expand_tabs(text, at, tab);
        at = at + Cells::of(&drawn);
        out.push((drawn, token));
    }
    out
}

fn sanitize_code_run(text: &str) -> Cow<'_, str> {
    if text
        .bytes()
        .all(|b| (0x20..0x7F).contains(&b) || b == b'\t')
        || !text.chars().any(super::cells::is_unsafe_code_char)
    {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(
            text.chars()
                .map(|ch| {
                    if super::cells::is_unsafe_code_char(ch) {
                        super::cells::UNPRINTABLE
                    } else {
                        ch
                    }
                })
                .collect(),
        )
    }
}

/// The colour each token class gets on the blue ground.
///
/// Colorer's console scheme for Far, in its sixteen colours, rather than every
/// class it has: Colorer separates a dozen classes, and on one background most
/// of those separations stop being legible before they stop being accurate.
/// These seven are the ones a reader actually navigates by.
pub(super) fn token_style(token: Token) -> Style {
    match token {
        Token::Normal => Style::default().fg(palette::CODE_NORMAL),
        // Types had been folded into Normal, so every `String`, `Vec`, `i32`
        // the tables already classify was drawn as a plain identifier — a whole
        // class computed and then discarded.
        Token::Type => Style::default().fg(palette::CODE_TYPE),
        Token::Keyword => Style::default()
            .fg(palette::CODE_KEYWORD)
            .add_modifier(Modifier::BOLD),
        Token::Str => Style::default().fg(palette::CODE_STRING),
        // Far paints switches the same colour it gives numbers.
        Token::Number | Token::Switch => Style::default().fg(palette::CODE_NUMBER),
        Token::Comment => Style::default().fg(palette::CODE_COMMENT),
        // A directive is not a comment: Colorer gives `def:Directive` its own
        // colour, and `#include` drawn comment-orange reads as dead code.
        Token::Meta => Style::default().fg(palette::CODE_DIRECTIVE),
        Token::Punctuation => Style::default().fg(palette::CODE_PUNCTUATION),
        Token::Tag => Style::default().fg(palette::CODE_TAG),
        Token::Url => Style::default().fg(palette::CODE_URL),
        Token::Uri => Style::default()
            .fg(palette::CODE_URI_FG)
            .bg(palette::CODE_URI_BG),
    }
}

/// Cut a already-styled line down to the horizontal window, run by run.
///
/// The window scrolls sideways in cells, and a highlighted line is many spans
/// rather than one string, so the window has to be walked across them —
/// `Paragraph::scroll` would take the whole line or none of it, and slicing
/// each span independently would drop the wrong ones.
pub(super) fn clip_runs<S: AsRef<str>>(
    runs: Vec<(S, Token)>,
    skip: Cells,
    width: Cells,
) -> Line<'static> {
    let mut spans = Vec::new();
    let mut to_skip = skip;
    let mut room = width;

    for (text, token) in runs {
        if room.is_zero() {
            break;
        }
        let clean = sanitize_code_run(text.as_ref());
        let text = clean.as_ref();
        let text = if to_skip.is_zero() {
            text
        } else {
            let (rest, dropped) = skip_cells(text, to_skip);
            to_skip = to_skip.saturating_sub(dropped);
            rest
        };
        if text.is_empty() {
            continue;
        }
        let (taken, used) = take_cells(text, room);
        room = room.saturating_sub(used);
        spans.push(Span::styled(taken, token_style(token)));
    }
    Line::from(spans)
}

/// The `rows` lines of `lines` from `top` down, highlighted and clipped to the
/// window `skip` cells in and `width` cells wide.
///
/// Slices the visible window here instead of via `Paragraph::scroll`: that
/// takes `u16`, so an offset past 65535 (one-line minified files) would wrap
/// around and draw text the cursor is not on. Cells, never bytes and never
/// chars.
///
/// Highlighting starts at line 0, not at `top`: a block comment opened above
/// the window still colours what is inside it, and a highlighter seeded at the
/// first visible line cannot know that. Only the visible runs are kept.
pub(super) fn highlighted_window(
    cache: &std::cell::RefCell<HighlightCache>,
    path: &std::path::Path,
    lines: &[String],
    top: usize,
    rows: usize,
    skip: Cells,
    width: Cells,
) -> Vec<Line<'static>> {
    let mut cache = cache.borrow_mut();
    cache.prepare(path, lines, top.saturating_add(rows));
    let limit = skip + width;
    lines
        .iter()
        .enumerate()
        .skip(top)
        .take(rows)
        .map(|(index, line)| {
            clip_runs(
                drawn_runs(cache.runs(index, line), TabSize::DEFAULT, limit),
                skip,
                width,
            )
        })
        .collect()
}
