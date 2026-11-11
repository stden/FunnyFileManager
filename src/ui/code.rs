//! Syntax-highlighted text on the blue ground, shared by the editor and the
//! viewer.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

#[cfg(test)]
use std::borrow::Cow;

use super::cells::clip::ColumnWindow;
use super::palette;
use super::{expand_tabs_to, Cells};
use crate::highlight::{
    cache::{HighlightCache, Span as CachedSpan},
    Token,
};
use crate::models::TabSize;

pub(super) fn editor_window(
    editor: &crate::models::Editor,
    top: usize,
    rows: usize,
    skip: Cells,
    width: Cells,
) -> Vec<Line<'_>> {
    let mut cache = editor.highlights.borrow_mut();
    cache.prepare_window(
        &editor.path,
        &editor.lines,
        top,
        top.saturating_add(rows),
        editor.lsp.revision.0,
        editor.take_dirty_span(),
    );
    editor
        .lines
        .iter()
        .enumerate()
        .skip(top)
        .take(rows)
        .map(|(index, line)| render_line(&cache, index, line, editor.tab_size, skip, width))
        .collect()
}

fn render_line<'a>(
    cache: &HighlightCache,
    index: usize,
    line: &'a str,
    tab: TabSize,
    skip: Cells,
    width: Cells,
) -> Line<'a> {
    if let Some(cached) = cache.span_slice(index) {
        let mut spans = Vec::with_capacity(cached.len());
        let mut window = ColumnWindow::new(skip, width);
        let mut at = Cells::ZERO;
        for &CachedSpan { start, end, token } in cached {
            if window.is_full() {
                break;
            }
            let text = line.get(start..end).unwrap_or_default();
            let (drawn, next_at) = expand_tabs_to(text, at, tab);
            at = next_at;
            if let Some(clipped) = window.clip(drawn) {
                spans.push(Span::styled(clipped, token_style(token)));
            }
        }
        return Line::from(spans);
    }
    render_plain_line(line, tab, skip, width)
}

fn render_plain_line(line: &str, tab: TabSize, skip: Cells, width: Cells) -> Line<'_> {
    let mut window = ColumnWindow::new(skip, width);
    let (drawn, _) = expand_tabs_to(line, Cells::ZERO, tab);
    match window.clip(drawn) {
        Some(clipped) => Line::from(vec![Span::styled(clipped, token_style(Token::Normal))]),
        None => Line::default(),
    }
}

#[cfg(test)]
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
pub(super) fn token_style(token: Token) -> Style {
    match token {
        Token::Normal => Style::default().fg(palette::CODE_NORMAL),
        Token::Type => Style::default().fg(palette::CODE_TYPE),
        Token::Keyword => Style::default()
            .fg(palette::CODE_KEYWORD)
            .add_modifier(Modifier::BOLD),
        Token::Str => Style::default().fg(palette::CODE_STRING),
        Token::Number | Token::Switch => Style::default().fg(palette::CODE_NUMBER),
        Token::Comment => Style::default().fg(palette::CODE_COMMENT),
        Token::Meta => Style::default().fg(palette::CODE_DIRECTIVE),
        Token::Punctuation => Style::default().fg(palette::CODE_PUNCTUATION),
        Token::Tag => Style::default().fg(palette::CODE_TAG),
        Token::Url => Style::default().fg(palette::CODE_URL),
        Token::Uri => Style::default()
            .fg(palette::CODE_URI_FG)
            .bg(palette::CODE_URI_BG),
    }
}

/// Cut an already-styled line down to the horizontal window, run by run.
#[cfg(test)]
pub(super) fn clip_runs<S: AsRef<str>>(
    runs: Vec<(S, Token)>,
    skip: Cells,
    width: Cells,
) -> Line<'static> {
    let mut spans = Vec::with_capacity(runs.len());
    let mut window = ColumnWindow::new(skip, width);
    for (text, token) in runs {
        if window.is_full() {
            break;
        }
        let clean = sanitize_code_run(text.as_ref());
        if let Some(clipped) = window.clip(clean) {
            spans.push(Span::styled(clipped.into_owned(), token_style(token)));
        }
    }
    Line::from(spans)
}

/// The `rows` lines of `lines` from `top` down, highlighted and clipped.
pub(super) fn highlighted_window<'a>(
    cache: &std::cell::RefCell<HighlightCache>,
    path: &std::path::Path,
    lines: &'a [String],
    top: usize,
    rows: usize,
    skip: Cells,
    width: Cells,
) -> Vec<Line<'a>> {
    let mut cache = cache.borrow_mut();
    cache.prepare_window(path, lines, top, top.saturating_add(rows), 0, None);
    lines
        .iter()
        .enumerate()
        .skip(top)
        .take(rows)
        .map(|(index, line)| render_line(&cache, index, line, TabSize::DEFAULT, skip, width))
        .collect()
}
