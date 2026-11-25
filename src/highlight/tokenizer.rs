//! Purpose: The per-line tokenizer: comments, strings, numbers, words, switches and variables.

mod scan;

use super::{Syntax, Token};

/// Whether the tokenizer is inside a multi-line block comment across lines.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) enum BlockState {
    #[default]
    Outside,
    Inside {
        closing_delimiter: &'static str,
    },
}

#[derive(Clone, Copy)]
struct TokenStep {
    byte_len: usize,
    token: Token,
}

impl TokenStep {
    const fn new(byte_len: usize, token: Token) -> Self {
        Self { byte_len, token }
    }
}

/// Split one line into token runs, updating `block` for the next line.
pub(super) fn tokenize<'a>(
    line: &'a str,
    syntax: Syntax,
    block: &mut BlockState,
) -> Vec<(&'a str, Token)> {
    if line.is_empty() {
        return Vec::new();
    }
    if let Some(token) = whole_line_token(line, syntax, *block) {
        return vec![(line, token)];
    }
    let mut runs = Vec::with_capacity((line.len() / 6).clamp(4, 64));
    let mut rest = line;
    while let Some(&byte) = rest.as_bytes().first() {
        let step = step_token(rest, byte, syntax, block);
        let (run, tail) = rest.split_at_checked(step.byte_len).unwrap_or((rest, ""));
        runs.push((run, step.token));
        rest = tail;
    }
    runs
}

fn step_token(rest: &str, byte: u8, syntax: Syntax, block: &mut BlockState) -> TokenStep {
    if let BlockState::Inside { closing_delimiter } = *block {
        let byte_len = rest.find(closing_delimiter).map_or(rest.len(), |offset| {
            *block = BlockState::Outside;
            offset + closing_delimiter.len()
        });
        return TokenStep::new(byte_len, Token::Comment);
    }
    if scan::is_ascii_ws(byte) {
        return TokenStep::new(scan::scan_whitespace(rest), Token::Normal);
    }
    if let Some(byte_len) = open_comment_length(rest, syntax, block) {
        return TokenStep::new(byte_len, Token::Comment);
    }
    if byte.is_ascii() {
        ascii_token(rest, byte, syntax)
    } else {
        unicode_token(rest, syntax)
    }
}

/// Sections and directives take precedence over individual tokens.
fn whole_line_token(line: &str, syntax: Syntax, block: BlockState) -> Option<Token> {
    if matches!(block, BlockState::Inside { .. }) {
        return None;
    }
    let trimmed = line.trim();
    if syntax.section_headers
        && trimmed.len() > 1
        && trimmed.starts_with('[')
        && trimmed.ends_with(']')
    {
        return Some(Token::Keyword);
    }
    syntax
        .meta_prefix
        .filter(|prefix| line.trim_start().starts_with(*prefix))
        .map(|_| Token::Meta)
}

fn open_comment_length(rest: &str, syntax: Syntax, block: &mut BlockState) -> Option<usize> {
    if syntax
        .line_comments
        .iter()
        .any(|marker| rest.starts_with(marker))
    {
        return Some(rest.len());
    }
    block_comment_length(rest, syntax, block)
}

/// Recognize an opening block comment and retain its continuation state.
fn block_comment_length(rest: &str, syntax: Syntax, block: &mut BlockState) -> Option<usize> {
    let &(open, close) = syntax
        .block_comments
        .iter()
        .find(|&&(open, _)| rest.starts_with(open))?;
    let body = rest.strip_prefix(open)?;
    Some(if let Some(offset) = body.find(close) {
        open.len() + offset + close.len()
    } else {
        *block = BlockState::Inside {
            closing_delimiter: close,
        };
        rest.len()
    })
}

fn ascii_token(rest: &str, byte: u8, syntax: Syntax) -> TokenStep {
    let ch = char::from(byte);
    if syntax.quotes.contains(&ch) {
        return TokenStep::new(scan::string_length(rest, byte), Token::Str);
    }
    if let Some(step) = prefixed_token(rest, syntax, ch) {
        return step;
    }
    if byte.is_ascii_digit() {
        return TokenStep::new(scan::scan_number(rest), Token::Number);
    }
    if byte.is_ascii_alphabetic() || byte == b'_' {
        let byte_len = scan::scan_word(rest, syntax.word_hyphens);
        let token = word_token(rest.get(..byte_len).unwrap_or(rest), syntax);
        return TokenStep::new(byte_len, token);
    }
    TokenStep::new(1, Token::Punctuation)
}

fn unicode_token(rest: &str, syntax: Syntax) -> TokenStep {
    let ch = rest.chars().next().unwrap_or(' ');
    if ch.is_alphabetic() {
        let byte_len = run_length(rest, |c| {
            c.is_alphanumeric() || c == '_' || (syntax.word_hyphens && c == '-')
        });
        let token = word_token(rest.get(..byte_len).unwrap_or(rest), syntax);
        return TokenStep::new(byte_len, token);
    }
    if ch.is_whitespace() {
        return TokenStep::new(run_length(rest, char::is_whitespace), Token::Normal);
    }
    TokenStep::new(ch.len_utf8(), Token::Punctuation)
}

fn word_token(word: &str, syntax: Syntax) -> Token {
    if syntax.keywords.binary_search(&word).is_ok() {
        Token::Keyword
    } else if syntax.types.binary_search(&word).is_ok() {
        Token::Type
    } else {
        Token::Normal
    }
}

fn prefixed_token(rest: &str, syntax: Syntax, ch: char) -> Option<TokenStep> {
    let tail = rest.strip_prefix(ch).unwrap_or_default();
    if syntax.switch_prefix == Some(ch) && tail.chars().next().is_some_and(char::is_alphanumeric) {
        return Some(TokenStep::new(
            ch.len_utf8() + run_length(tail, char::is_alphanumeric),
            Token::Switch,
        ));
    }
    if syntax.variable_prefix == Some(ch) {
        let name = run_length(tail, |c| c.is_alphanumeric() || c == '_' || c == ':');
        if name > 0 {
            return Some(TokenStep::new(ch.len_utf8() + name, Token::Type));
        }
    }
    None
}

/// Byte length of the leading run of characters satisfying `keep`.
pub(super) fn run_length(text: &str, keep: impl Fn(char) -> bool) -> usize {
    text.char_indices()
        .find(|(_, ch)| !keep(*ch))
        .map_or(text.len(), |(offset, _)| offset)
}
