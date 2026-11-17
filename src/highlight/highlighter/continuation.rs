//! Purpose: Cross-line continuation state checkpoints for incremental highlighting.

use super::{dotenv, markup, tokenize, BlockState, Highlighter, Scanner};

/// Compact cross-line state carried between adjacent lines by the built-in scanner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::highlight) struct Continuation(Kind);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Plain,
    Tokens(BlockState),
    Markdown(bool),
    Dotenv(dotenv::State),
    Markup(markup::State),
}

impl Highlighter {
    /// Snapshot the current cross-line continuation state when using the built-in
    /// line-by-line scanner (`None` when backed by whole-file Colorer regions).
    #[must_use]
    #[cfg_attr(
        not(feature = "colorer"),
        expect(
            clippy::unnecessary_wraps,
            reason = "the colorer build returns None when backed by whole-file Colorer regions"
        )
    )]
    pub(in crate::highlight) fn continuation(&self) -> Option<Continuation> {
        #[cfg(feature = "colorer")]
        if !self.regions.is_empty() {
            return None;
        }
        Some(Continuation(match &self.scanner {
            Scanner::Plain => Kind::Plain,
            Scanner::Tokens { block, .. } => Kind::Tokens(*block),
            Scanner::Markdown { in_fence } => Kind::Markdown(*in_fence),
            Scanner::Dotenv(state) => Kind::Dotenv(*state),
            Scanner::Markup(state) => Kind::Markup(*state),
        }))
    }

    /// Restore the scanner's cross-line state before re-lexing a modified line.
    pub(in crate::highlight) fn resume(&mut self, state: Continuation) {
        match (&mut self.scanner, state.0) {
            (Scanner::Tokens { block, .. }, Kind::Tokens(next)) => *block = next,
            (Scanner::Markdown { in_fence }, Kind::Markdown(next)) => *in_fence = next,
            (Scanner::Dotenv(current), Kind::Dotenv(next)) => *current = next,
            (Scanner::Markup(current), Kind::Markup(next)) => *current = next,
            _ => {}
        }
    }

    /// Fast-forward cross-line continuation state across an off-screen line without
    /// building token runs when the line cannot alter cross-line state.
    pub(in crate::highlight) fn advance_only(&mut self, line: &str) {
        match &mut self.scanner {
            Scanner::Plain => {}
            Scanner::Tokens { syntax, block } => advance_tokens(line, *syntax, block),
            Scanner::Markdown { in_fence } => {
                let trimmed = line.trim_start();
                if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                    *in_fence = !*in_fence;
                }
            }
            Scanner::Dotenv(state) => {
                if !matches!(*state, dotenv::State::Unquoted)
                    || line.contains('\'')
                    || line.contains('"')
                {
                    drop(state.line(line));
                }
            }
            Scanner::Markup(state) => state.advance(line),
        }
    }
}

fn advance_tokens(line: &str, syntax: super::Syntax, block: &mut BlockState) {
    if syntax.block_comments.is_empty() {
        return;
    }
    let might_change = match *block {
        BlockState::Inside { closing_delimiter } => {
            has_delimiter(line.as_bytes(), closing_delimiter)
        }
        BlockState::Outside => has_any_opening(line.as_bytes(), syntax.block_comments),
    };
    if might_change {
        drop(tokenize(line, syntax, block));
    }
}

fn has_any_opening(bytes: &[u8], pairs: &[(&str, &str)]) -> bool {
    match pairs {
        [(a, _), (b, _)] => has_either_delimiter(bytes, a, b),
        _ => pairs.iter().any(|&(open, _)| has_delimiter(bytes, open)),
    }
}

fn has_either_delimiter(bytes: &[u8], a: &str, b: &str) -> bool {
    a.as_bytes()
        .first()
        .zip(b.as_bytes().first())
        .is_some_and(|(&b0, &b1)| {
            memchr::memchr2(b0, b1, bytes).is_some_and(|pos| {
                let tail = bytes.get(pos..).unwrap_or_default();
                has_delimiter(tail, a) || has_delimiter(tail, b)
            })
        })
}

fn has_delimiter(bytes: &[u8], delim: &str) -> bool {
    delim
        .as_bytes()
        .split_first()
        .is_some_and(|(&first, rest)| contains_after_first(bytes, first, rest))
}

fn contains_after_first(mut slice: &[u8], first: u8, rest: &[u8]) -> bool {
    while let Some(pos) = memchr::memchr(first, slice) {
        let after = slice.get(pos + 1..).unwrap_or_default();
        if after.starts_with(rest) {
            return true;
        }
        slice = after;
    }
    false
}
