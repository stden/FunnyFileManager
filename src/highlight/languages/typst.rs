//! Purpose: The Typst word list, kept apart from the other language tables.

use super::super::{LineMode, Syntax};

/// Typst: content markup language with `#`-prefixed meta commands.
pub(super) const TYPST: Syntax = Syntax {
    mode: LineMode::Tokens,
    line_comments: &["//"],
    block_comments: &[("/*", "*/")],
    quotes: &['"'],
    keywords: &[
        "and", "as", "auto", "break", "continue", "else", "false", "for", "if", "import", "in",
        "include", "let", "none", "not", "or", "return", "set", "show", "true", "while",
    ],
    types: &[
        "align",
        "array",
        "bool",
        "color",
        "content",
        "dict",
        "direction",
        "float",
        "fraction",
        "int",
        "length",
        "ratio",
        "relative",
        "str",
    ],
    meta_prefix: Some('#'),
    section_headers: false,
    variable_prefix: None,
    word_hyphens: false,
    switch_prefix: None,
};
