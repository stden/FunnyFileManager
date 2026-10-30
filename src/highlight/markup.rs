//! Purpose: Stateful XML/HTML/SVG line scanner matching `FarColorer`'s console scheme.
//!
//! Tracks multi-line tags (`<project ...>`) and comments (`<!-- ... -->`) across
//! lines, keeps element names (`<jackson-2-bom.version>`) and character data
//! (`26.10.13`) intact instead of splitting numbers and hyphens as code tokens,
//! and highlights attribute namespaces, URLs, and `${...}` interpolations.

mod attr;
mod text;

use super::{run_length, Token};
use attr::scan_tag_body;
use text::{scan_comment, scan_text_segment};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) enum State {
    #[default]
    Outside,
    InTag,
    InComment,
}

impl State {
    pub(super) fn line<'a>(&mut self, line: &'a str) -> Vec<(&'a str, Token)> {
        let mut runs = Vec::new();
        let mut rest = line;
        while !rest.is_empty() {
            rest = match self {
                Self::Outside => self.scan_outside(rest, &mut runs),
                Self::InTag => scan_tag_body(rest, self, &mut runs),
                Self::InComment => scan_comment(rest, self, &mut runs),
            };
        }
        runs
    }

    fn scan_outside<'a>(&mut self, rest: &'a str, runs: &mut Vec<(&'a str, Token)>) -> &'a str {
        if rest.starts_with("<!--") {
            *self = Self::InComment;
            return scan_comment(rest, self, runs);
        }
        if rest.starts_with('<') {
            return self.scan_tag_open(rest, runs);
        }
        let text_len = rest.find('<').unwrap_or(rest.len());
        let (text, after) = rest.split_at(text_len);
        scan_text_segment(text, runs);
        after
    }

    fn scan_tag_open<'a>(&mut self, rest: &'a str, runs: &mut Vec<(&'a str, Token)>) -> &'a str {
        let bracket_len =
            if rest.starts_with("</") || rest.starts_with("<?") || rest.starts_with("<!") {
                2
            } else {
                1
            };
        let (bracket, after_bracket) = rest.split_at(bracket_len);
        let name_len = run_length(after_bracket, is_tag_name_char);
        if name_len == 0 && bracket == "<" {
            push(runs, bracket, Token::Punctuation);
            return after_bracket;
        }
        let (name, after_name) = after_bracket.split_at(name_len);
        push(runs, bracket, Token::Punctuation);
        push(runs, name, Token::Tag);
        *self = Self::InTag;
        after_name
    }
}

pub(super) fn is_tag_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':')
}

pub(super) fn push<'a>(runs: &mut Vec<(&'a str, Token)>, text: &'a str, token: Token) {
    if !text.is_empty() {
        runs.push((text, token));
    }
}
