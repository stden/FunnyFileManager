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

pub mod cache;
mod dotenv;
mod formats;
mod highlighter;
mod languages;
mod markup;
#[cfg(feature = "colorer")]
mod regions;
mod shells;
mod tokenizer;

pub use highlighter::Highlighter;
pub use languages::syntax_for;
#[cfg(feature = "colorer")]
use regions::paint;
use tokenizer::{run_length, tokenize};

#[cfg(test)]
thread_local! {
    pub(crate) static FILE_PASSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    pub(crate) static LINE_PASSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    pub(crate) static HASH_PASSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
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
    /// Markup element names (`<groupId>`, `</project>`) and `=` delimiters:
    /// Colorer's light grey (`#C0C0C0`).
    Tag,
    /// Hyperlinks in markup attributes/comments and unclosed multi-line tag
    /// openings (`<project`): Colorer's bright red (`#FF0000`).
    Url,
    /// Bare URI in markup element text (`<url>http://...</url>`): Colorer's
    /// black on dark-red background (`#000000` on `#800000`).
    Uri,
}

/// One language's lexical surface: what starts a comment, what quotes a
/// string, and which words are reserved.
#[derive(Debug, Clone, Copy)]
pub struct Syntax {
    mode: LineMode,
    line_comments: &'static [&'static str],
    block_comments: &'static [(&'static str, &'static str)],
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
    Markup,
}

#[cfg(test)]
mod tests;
