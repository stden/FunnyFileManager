//! Purpose: The [`Highlighter`] a viewer or editor walks a file with — Colorer's
//! regions when it has them, else the built-in scanner of the file's language.

use std::path::Path;

#[cfg(test)]
use super::FILE_PASSES;
#[cfg(feature = "colorer")]
use super::{colorer, paint};
use super::{dotenv, syntax_for, tokenize, LineMode, Syntax, Token};

#[derive(Debug)]
enum Scanner {
    Plain,
    Tokens { syntax: Syntax, in_block: bool },
    Markdown { in_fence: bool },
    Dotenv(dotenv::State),
}

/// Highlights a file's lines in order with language-specific continuation state.
#[derive(Debug)]
pub struct Highlighter {
    scanner: Scanner,
    /// Regions Colorer found, by line, when it is available and knows the
    /// language. Empty otherwise, and the tokenizer below does the work.
    #[cfg(feature = "colorer")]
    pub(super) regions: Vec<Vec<colorer::Span>>,
    /// How far into `regions` [`Highlighter::line`] has read, since it is
    /// called once per line in order.
    #[cfg(feature = "colorer")]
    at: usize,
}

impl Highlighter {
    #[must_use]
    pub fn new(path: &Path) -> Self {
        let scanner = syntax_for(path).map_or(Scanner::Plain, |syntax| match syntax.mode {
            LineMode::Tokens => Scanner::Tokens {
                syntax,
                in_block: false,
            },
            LineMode::Markdown => Scanner::Markdown { in_fence: false },
            LineMode::Dotenv => Scanner::Dotenv(dotenv::State::default()),
        });
        Self {
            scanner,
            #[cfg(feature = "colorer")]
            regions: Vec::new(),
            #[cfg(feature = "colorer")]
            at: 0,
        }
    }

    /// A highlighter for a whole file, using Colorer when it can.
    ///
    /// Colorer parses the file in one pass rather than line by line, because a
    /// scheme's state can reach back further than the one bit of block-comment
    /// context the built-in tokenizer carries. The lines are therefore handed
    /// over up front, and [`Self::line`] afterwards just reads off the answer.
    ///
    /// Falls back to the tokenizer whenever Colorer is absent, has no scheme
    /// for the file, or finds nothing — a missing scheme database must cost
    /// colour, never the file.
    #[must_use]
    /// Pick an engine for `path`: the table above when it knows the language,
    /// Colorer when it does not.
    ///
    /// That order is measured, not assumed. Colorer brings hundreds of
    /// languages, which is why it is here at all — but for the handful this
    /// table covers, its stock schemes are the weaker of the two, and by a
    /// wide margin. Colouring one line of each, counting the characters that
    /// come out as something other than plain text:
    ///
    /// | | Colorer | this table |
    /// |---|---|---|
    /// | Rust | 46% | 91% |
    /// | PowerShell | 22% | 97% |
    /// | Python | 82% | 93% |
    /// | JSON | 61% | 100% |
    /// | Shell | 8% | 100% |
    /// | C++ | 82% | 80% |
    ///
    /// The gaps are not close calls. Colorer's `rust.hrc` is written against
    /// the 1.0-beta grammar and has no string rule at all; its PowerShell
    /// scheme colours a string's quotes but not the text between them, and
    /// treats `$variables` and every verb-noun cmdlet as bare words. C++ is
    /// the one near-tie, and it goes to the table for consistency rather than
    /// being special-cased on a two-point difference.
    pub fn for_file(path: &Path, lines: &[String]) -> Self {
        #[cfg(test)]
        FILE_PASSES.set(FILE_PASSES.get() + 1);
        #[cfg_attr(
            not(feature = "colorer"),
            expect(unused_mut, reason = "only the colorer build fills the regions")
        )]
        let mut highlighter = Self::new(path);
        #[cfg(feature = "colorer")]
        if matches!(highlighter.scanner, Scanner::Plain) {
            if let Some(engine) =
                colorer::Colorer::shared().filter(|engine| engine.file_type(path).is_some())
            {
                highlighter.regions = engine.parse(path, lines);
            }
        }
        let _ = lines;
        highlighter
    }

    /// Whether anything will actually be coloured, so the caller can skip the
    /// per-line work for a plain-text file.
    ///
    /// True when the built-in table knows the language, or when Colorer filled
    /// regions for a language the table does not cover. A `.txt` with neither
    /// stays false, and the viewer paints it in one colour without walking
    /// from line 0.
    #[must_use]
    pub fn is_active(&self) -> bool {
        let builtin = !matches!(self.scanner, Scanner::Plain);
        #[cfg(feature = "colorer")]
        {
            builtin || !self.regions.is_empty()
        }
        #[cfg(not(feature = "colorer"))]
        {
            builtin
        }
    }

    /// Classify one line, advancing the cross-line state.
    ///
    /// Returns `(text, token)` runs covering the whole line in order, so the
    /// caller can concatenate them back into the original.
    pub fn line<'a>(&mut self, line: &'a str) -> Vec<(&'a str, Token)> {
        #[cfg(feature = "colorer")]
        {
            let at = self.at;
            self.at += 1;
            if let Some(spans) = self.regions.get(at).filter(|spans| !spans.is_empty()) {
                return paint(line, spans);
            }
        }
        match &mut self.scanner {
            Scanner::Plain => vec![(line, Token::Normal)],
            Scanner::Tokens { syntax, in_block } => tokenize(line, *syntax, in_block),
            Scanner::Markdown { in_fence } => Self::markdown_line(line, in_fence),
            Scanner::Dotenv(state) => state.line(line),
        }
    }

    /// Markdown by line shape: headings, fences, quotes and list bullets are
    /// what a reader scans for, and none of them is a token.
    fn markdown_line<'a>(line: &'a str, in_fence: &mut bool) -> Vec<(&'a str, Token)> {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            *in_fence = !*in_fence;
            return vec![(line, Token::Comment)];
        }
        if *in_fence {
            return vec![(line, Token::Str)];
        }
        if trimmed.starts_with('#') {
            return vec![(line, Token::Keyword)];
        }
        if trimmed.starts_with('>') {
            return vec![(line, Token::Comment)];
        }
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("| ") {
            // The indent and the one-byte marker, then the item's text.
            if let Some((bullet, text)) = line.split_at_checked(line.len() - trimmed.len() + 1) {
                return vec![(bullet, Token::Meta), (text, Token::Normal)];
            }
        }
        vec![(line, Token::Normal)]
    }
}
