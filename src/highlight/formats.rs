//! Purpose: Language tables for data and markup formats: configuration files,
//! JSON, HTML/XML, Markdown and dotenv.

use super::{LineMode, Syntax};

pub(super) const CONFIG: Syntax = Syntax {
    mode: LineMode::Tokens,
    line_comments: &["#", ";"],
    block_comment: None,
    quotes: &['"', '\''],
    keywords: &["false", "no", "null", "off", "on", "true", "yes"],
    types: &[],
    meta_prefix: None,
    section_headers: true,
    variable_prefix: None,
    word_hyphens: false,
    switch_prefix: None,
};

pub(super) const JSON: Syntax = Syntax {
    mode: LineMode::Tokens,
    line_comments: &[],
    block_comment: None,
    quotes: &['"'],
    keywords: &["false", "null", "true"],
    types: &[],
    meta_prefix: None,
    section_headers: false,
    variable_prefix: None,
    word_hyphens: false,
    switch_prefix: None,
};

pub(super) const MARKUP: Syntax = Syntax {
    mode: LineMode::Markup,
    ..CONFIG
};

/// Structured formats select their own scanner instead of a token table.
pub(super) const MARKDOWN: Syntax = Syntax {
    mode: LineMode::Markdown,
    ..CONFIG
};

pub(super) const DOTENV: Syntax = Syntax {
    mode: LineMode::Dotenv,
    ..CONFIG
};
