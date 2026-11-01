//! Purpose: The per-line tokenizer: comments, strings, numbers, words, switches and variables.

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

/// Split one line into token runs, updating `block` for the next line.
pub(super) fn tokenize<'a>(
    line: &'a str,
    syntax: Syntax,
    block: &mut BlockState,
) -> Vec<(&'a str, Token)> {
    if let Some(token) = whole_line_token(line, syntax, *block) {
        return vec![(line, token)];
    }
    let mut runs = Vec::new();
    let mut rest = line;
    while let Some(first) = rest.chars().next() {
        let (length, token) = comment_length(rest, syntax, block).map_or_else(
            || next_token(rest, first, syntax),
            |length| (length, Token::Comment),
        );
        // Every length is counted in whole characters, so the split succeeds;
        // were one ever off, the rest of the line stays in this run.
        let (run, tail) = rest.split_at_checked(length).unwrap_or((rest, ""));
        runs.push((run, token));
        rest = tail;
    }
    runs
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

fn comment_length(rest: &str, syntax: Syntax, block: &mut BlockState) -> Option<usize> {
    if let BlockState::Inside { closing_delimiter } = *block {
        return Some(rest.find(closing_delimiter).map_or(rest.len(), |offset| {
            *block = BlockState::Outside;
            offset + closing_delimiter.len()
        }));
    }
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

/// Read one non-comment token from `rest`, whose first character is `ch`.
fn next_token(rest: &str, ch: char, syntax: Syntax) -> (usize, Token) {
    if syntax.quotes.contains(&ch) {
        return (string_length(rest, ch), Token::Str);
    }

    if let Some(token) = prefixed_token(rest, syntax, ch) {
        return token;
    }

    if ch.is_ascii_digit() {
        return (
            run_length(rest, |c| c.is_alphanumeric() || c == '.' || c == '_'),
            Token::Number,
        );
    }

    if ch.is_alphabetic() || ch == '_' {
        let length = run_length(rest, |c| {
            c.is_alphanumeric() || c == '_' || (syntax.word_hyphens && c == '-')
        });
        let token = word_token(rest.get(..length).unwrap_or(rest), syntax);
        return (length, token);
    }

    // Retain whitespace in a single run so indentation does not multiply spans.
    if ch.is_whitespace() {
        return (run_length(rest, char::is_whitespace), Token::Normal);
    }
    (ch.len_utf8(), Token::Punctuation)
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

fn prefixed_token(rest: &str, syntax: Syntax, ch: char) -> Option<(usize, Token)> {
    let tail = rest.strip_prefix(ch).unwrap_or_default();
    if syntax.switch_prefix == Some(ch) && tail.chars().next().is_some_and(char::is_alphanumeric) {
        return Some((
            ch.len_utf8() + run_length(tail, char::is_alphanumeric),
            Token::Switch,
        ));
    }
    if syntax.variable_prefix == Some(ch) {
        let name = run_length(tail, |c| c.is_alphanumeric() || c == '_' || c == ':');
        if name > 0 {
            return Some((ch.len_utf8() + name, Token::Type));
        }
    }
    None
}

/// Byte length of the quoted string starting at `text[0]`, including both
/// quotes — or the rest of the line when the closing quote is missing, which
/// an editor sees constantly because the user is still typing it.
fn string_length(text: &str, quote: char) -> usize {
    let mut escaped = false;
    for (offset, ch) in text.char_indices().skip(1) {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
        } else if ch == quote {
            return offset + ch.len_utf8();
        }
    }
    text.len()
}

/// Byte length of the leading run of characters satisfying `keep`.
pub(super) fn run_length(text: &str, keep: impl Fn(char) -> bool) -> usize {
    text.char_indices()
        .find(|(_, ch)| !keep(*ch))
        .map_or(text.len(), |(offset, _)| offset)
}
