//! Purpose: What a Colorer region is to the editor — a [`Span`] of one line,
//! and the token class a region's name maps onto.

use crate::highlight::Token;

/// One region Colorer found: a half-open character range and what it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub token: Token,
}

/// Map one of Colorer's region names onto a token class.
///
/// Colorer's vocabulary is far larger than the editor's palette — every
/// language adds its own `python:pyKeyword`, `rust:rustMacro` — but the names
/// are systematic: a language region is conventionally the `def:` one with a
/// prefix. Matching on the *suffix* therefore covers languages this code has
/// never heard of, which is the point of using Colorer at all.
#[must_use]
pub fn region_token(region: &str) -> Token {
    // `python:pyKeyword` → `pyKeyword`; `def:Keyword` → `Keyword`.
    let tail = region.rsplit(':').next().unwrap_or(region);
    let lower = tail.to_ascii_lowercase();

    // Order matters: `CommentContent` must be a comment before `Content`
    // matches anything else, and `StringEdge` a string before `Edge` does.
    for (needle, token) in [
        ("comment", Token::Comment),
        ("string", Token::Str),
        ("character", Token::Str),
        ("number", Token::Number),
        ("keyword", Token::Keyword),
        ("word", Token::Keyword),
        ("directive", Token::Meta),
        ("preprocessor", Token::Meta),
        ("outline", Token::Meta),
        ("type", Token::Type),
        ("class", Token::Type),
        ("function", Token::Type),
        ("symbol", Token::Punctuation),
        ("operator", Token::Punctuation),
        ("delimiter", Token::Punctuation),
        ("pair", Token::Punctuation),
    ] {
        if lower.contains(needle) {
            return token;
        }
    }
    Token::Normal
}
