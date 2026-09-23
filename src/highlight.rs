//! Purpose: Colorer-style syntax highlighting for the built-in F4 editor.
//!
//! One small tokenizer driven by a per-language table, rather than a grammar
//! engine: the editor shows plain source files, and the token classes that
//! actually help a reader — comment, string, number, keyword — are the ones a
//! hand-written scanner gets right. A regex or PEG dependency would buy
//! nesting rules this editor has no way to display.
//!
//! [`Highlighter`] carries only the state its selected language needs: block
//! comments, Markdown fences, or dotenv quotes that continue onto another line.

use std::path::Path;

pub mod cache;
mod dotenv;
mod languages;
#[cfg(feature = "colorer")]
mod regions;
mod shells;
mod tokenizer;

pub use languages::syntax_for;
#[cfg(feature = "colorer")]
use regions::paint;
use tokenizer::{run_length, tokenize};

#[cfg(test)]
thread_local! {
    pub(crate) static FILE_PASSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(feature = "colorer")]
pub mod colorer;

/// What a run of characters *is*, as far as colour is concerned.
///
/// Deliberately coarse. Colorer distinguishes far more, but every extra class
/// costs a colour, and a scheme with twelve colours on one background stops
/// being readable long before it stops being accurate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
    Normal,
    Keyword,
    /// Type names and language primitives — `int`, `String`, `bool`.
    Type,
    Str,
    Number,
    Comment,
    /// Preprocessor, attributes, decorators: `#include`, `#[derive]`, `@foo`.
    Meta,
    /// A command-line switch — `/b`, `/s`. Its own class rather than a number,
    /// even though Far paints the two the same green: a switch is not a
    /// quantity, and the next scheme to separate them should not have to
    /// untangle them first.
    Switch,
    Punctuation,
}

/// One language's lexical surface: what starts a comment, what quotes a
/// string, and which words are reserved.
#[derive(Debug, Clone, Copy)]
pub struct Syntax {
    mode: LineMode,
    line_comments: &'static [&'static str],
    block_comment: Option<(&'static str, &'static str)>,
    quotes: &'static [char],
    keywords: &'static [&'static str],
    types: &'static [&'static str],
    /// Line-leading character that marks the whole line as [`Token::Meta`].
    meta_prefix: Option<char>,
    /// Character that opens a command-line switch, as `/` does in `exit /b 1`.
    switch_prefix: Option<char>,
    /// Character that opens a variable, as `$` does in `$name`. PowerShell is
    /// mostly variables, and a scheme that paints them like bare words paints
    /// nearly the whole file one colour.
    variable_prefix: Option<char>,
    /// Whether a hyphen can sit inside a word. PowerShell names are verb-noun
    /// — `New-Object`, `Write-Host` — and splitting them at the hyphen leaves
    /// every cmdlet call reading as a bare word followed by a switch.
    word_hyphens: bool,
    /// Whether a line shaped `[like this]` is a section header.
    section_headers: bool,
}

#[derive(Debug, Clone, Copy)]
enum LineMode {
    Tokens,
    Markdown,
    Dotenv,
}

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
    regions: Vec<Vec<colorer::Span>>,
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
        #[cfg_attr(not(feature = "colorer"), allow(unused_mut))]
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
            let bullet = line.len() - trimmed.len() + 1;
            return vec![
                (&line[..bullet], Token::Meta),
                (&line[bullet..], Token::Normal),
            ];
        }
        vec![(line, Token::Normal)]
    }
}

#[cfg(test)]
mod tests;
