//! Purpose: Cross-line continuation state checkpoints for incremental highlighting.

use super::{dotenv, markup, BlockState, Highlighter, Scanner};

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
}
