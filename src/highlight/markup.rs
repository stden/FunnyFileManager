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

struct TagOpen {
    bracket_len: usize,
    name_len: usize,
    enters_tag: bool,
}

/// Inspect a `<` prefix (`<tag`, `</tag`, `<?pi`, `<!DOCTYPE`, or a lone `<`).
fn parse_tag_open(at_lt: &[u8]) -> TagOpen {
    let bracket_len = if matches!(at_lt.get(1), Some(b'/' | b'?' | b'!')) {
        2
    } else {
        1
    };
    let after_bracket = at_lt.get(bracket_len..).unwrap_or_default();
    let name_len = after_bracket
        .iter()
        .take_while(|&&b| is_tag_name_byte(b))
        .count();
    let enters_tag = !(bracket_len == 1 && name_len == 0);
    TagOpen {
        bracket_len,
        name_len,
        enters_tag,
    }
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
        let open = parse_tag_open(rest.as_bytes());
        let (bracket, after_bracket) = rest.split_at(open.bracket_len);
        push(runs, bracket, Token::Punctuation);
        if !open.enters_tag {
            return after_bracket;
        }
        let (name, after_name) = after_bracket.split_at(open.name_len);
        push(runs, name, Token::Tag);
        *self = Self::InTag;
        after_name
    }

    /// Fast-forward markup continuation state across `line` without allocating token runs.
    pub(super) fn advance(&mut self, line: &str) {
        let bytes = line.as_bytes();
        let mut pos = 0;
        while let Some(rest) = bytes.get(pos..) {
            if rest.is_empty() {
                break;
            }
            pos += match self {
                Self::Outside => advance_outside(rest, self),
                Self::InTag => advance_tag(rest, self),
                Self::InComment => advance_comment(rest, self),
            };
        }
    }
}

fn advance_outside(bytes: &[u8], state: &mut State) -> usize {
    let Some(lt) = memchr::memchr(b'<', bytes) else {
        return bytes.len();
    };
    let at_lt = bytes.get(lt..).unwrap_or_default();
    if at_lt.starts_with(b"<!--") {
        *state = State::InComment;
        return lt;
    }
    let open = parse_tag_open(at_lt);
    if open.enters_tag {
        *state = State::InTag;
    }
    lt + open.bracket_len + open.name_len
}

fn advance_tag(bytes: &[u8], state: &mut State) -> usize {
    let Some(hit) = memchr::memchr3(b'"', b'\'', b'>', bytes) else {
        return bytes.len();
    };
    if bytes.get(hit) == Some(&b'>') {
        *state = State::Outside;
        return hit + 1;
    }
    let quote = bytes.get(hit).copied().unwrap_or(b'"');
    let after_open_quote = hit + 1;
    let quoted_tail = bytes.get(after_open_quote..).unwrap_or_default();
    memchr::memchr(quote, quoted_tail).map_or(bytes.len(), |close| after_open_quote + close + 1)
}

fn advance_comment(bytes: &[u8], state: &mut State) -> usize {
    if let Some(end) = memchr::memmem::find(bytes, b"-->") {
        *state = State::Outside;
        return end + 3;
    }
    bytes.len()
}

const fn is_tag_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':')
}

pub(super) fn is_tag_name_char(ch: char) -> bool {
    u8::try_from(ch).is_ok_and(is_tag_name_byte)
}

pub(super) fn push<'a>(runs: &mut Vec<(&'a str, Token)>, text: &'a str, token: Token) {
    if !text.is_empty() {
        runs.push((text, token));
    }
}
