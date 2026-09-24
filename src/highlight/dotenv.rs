//! Dotenv assignment highlighting, independent of shell execution and Colorer.
//!
//! Values are text. Quotes can span lines; hashes inside quotes remain text,
//! and references are highlighted without expanding or reading their values.

use std::ops::Range;

use super::{run_length, Token};

#[derive(Debug, Default)]
pub(super) enum State {
    #[default]
    Unquoted,
    SingleQuoted,
    DoubleQuoted,
}

impl State {
    pub(super) fn line<'a>(&mut self, line: &'a str) -> Vec<(&'a str, Token)> {
        let mut runs = Vec::new();
        let Some(at) = self.value_start(line, &mut runs) else {
            return runs;
        };
        ValueScanner {
            line,
            state: self,
            runs,
            at,
            start: at,
            value_start: at,
        }
        .scan()
    }

    fn value_start<'a>(&self, line: &'a str, runs: &mut Vec<(&'a str, Token)>) -> Option<usize> {
        if !matches!(self, Self::Unquoted) {
            return Some(0);
        }
        let (indent, trimmed) = split_run(line, char::is_whitespace);
        if trimmed.starts_with('#') {
            push(runs, indent, Token::Normal);
            runs.push((trimmed, Token::Comment));
            return None;
        }
        let start = assignment(line, runs);
        if start.is_none() {
            push(runs, line, Token::Normal);
        }
        start
    }
}

/// Tracks byte boundaries while preserving quote state between lines.
struct ValueScanner<'line, 'state> {
    line: &'line str,
    state: &'state mut State,
    runs: Vec<(&'line str, Token)>,
    at: usize,
    start: usize,
    value_start: usize,
}

impl<'line> ValueScanner<'line, '_> {
    fn scan(mut self) -> Vec<(&'line str, Token)> {
        while let Some(ch) = self.rest().chars().next() {
            if ch == '\\' {
                self.at += ch.len_utf8();
                self.at += self.rest().chars().next().map_or(0, char::len_utf8);
            } else if matches!(self.state, State::Unquoted) && ch == '#' {
                self.emit(self.line.len() - self.at, Token::Comment);
                return self.runs;
            } else if !self.emit_special(ch) {
                self.advance_quote(ch);
                self.at += ch.len_utf8();
            }
        }
        let unfinished = self.text(self.start..self.line.len());
        push(&mut self.runs, unfinished, Token::Str);
        self.runs
    }

    /// The part of the line not scanned yet.
    fn rest(&self) -> &'line str {
        self.text(self.at..self.line.len())
    }

    /// The line's bytes in `range`. The scanner only stops on character
    /// boundaries, so this is never cut short.
    fn text(&self, range: Range<usize>) -> &'line str {
        self.line.get(range).unwrap_or_default()
    }

    fn emit_special(&mut self, ch: char) -> bool {
        let rest = self.rest();
        let token = match (&*self.state, ch) {
            (State::Unquoted | State::DoubleQuoted, '$') => {
                reference_length(rest).map(|length| (length, Token::Type))
            }
            (State::Unquoted, ch) if ch.is_whitespace() => {
                Some((run_length(rest, char::is_whitespace), Token::Normal))
            }
            _ => None,
        };
        if let Some((length, token)) = token {
            self.emit(length, token);
            return true;
        }
        false
    }

    fn emit(&mut self, length: usize, token: Token) {
        let (before, emitted) = (
            self.text(self.start..self.at),
            self.text(self.at..self.at + length),
        );
        push(&mut self.runs, before, Token::Str);
        self.runs.push((emitted, token));
        self.at += length;
        self.start = self.at;
    }

    fn advance_quote(&mut self, ch: char) {
        match (&*self.state, ch) {
            (State::Unquoted, '\'') if self.at == self.value_start => {
                *self.state = State::SingleQuoted;
            }
            (State::Unquoted, '"') if self.at == self.value_start => {
                *self.state = State::DoubleQuoted;
            }
            (State::SingleQuoted, '\'') | (State::DoubleQuoted, '"') => {
                *self.state = State::Unquoted;
            }
            _ => {}
        }
    }
}

/// Classify only a complete assignment prefix, leaving malformed input intact.
fn assignment<'a>(line: &'a str, runs: &mut Vec<(&'a str, Token)>) -> Option<usize> {
    let (prefix, after_equals) = line.split_once('=')?;
    let (indent, declaration) = split_run(prefix, char::is_whitespace);
    let (export_gap, key_part) = match declaration.strip_prefix("export") {
        Some(tail) if tail.starts_with(char::is_whitespace) => {
            let (gap, key_part) = split_run(tail, char::is_whitespace);
            (Some(gap), key_part)
        }
        _ => (None, declaration),
    };
    let key = key_part.trim_end();
    let padding = key_part.strip_prefix(key).unwrap_or_default();
    if !valid_name(key) {
        return None;
    }

    push(runs, indent, Token::Normal);
    if let Some(gap) = export_gap {
        runs.push(("export", Token::Keyword));
        push(runs, gap, Token::Normal);
    }
    runs.push((key, Token::Type));
    push(runs, padding, Token::Normal);
    runs.push(("=", Token::Punctuation));
    let (value_indent, value) = split_run(after_equals, char::is_whitespace);
    push(runs, value_indent, Token::Normal);
    Some(line.len() - value.len())
}

fn valid_name(name: &str) -> bool {
    name.as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn reference_length(text: &str) -> Option<usize> {
    let tail = text.strip_prefix('$')?;
    if let Some(braced) = tail.strip_prefix('{') {
        let (expression, _) = braced.split_once('}')?;
        let (name, _) = split_run(expression, is_name_char);
        // `$`, `{`, the expression and `}`.
        return valid_name(name).then_some(expression.len() + 3);
    }
    let (name, _) = split_run(tail, is_name_char);
    valid_name(name).then_some(name.len() + 1)
}

fn is_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

/// Split `text` after its leading run of characters satisfying `keep`.
fn split_run(text: &str, keep: impl Fn(char) -> bool) -> (&str, &str) {
    let rest = text.trim_start_matches(keep);
    // `rest` is a suffix of `text`, so the strip always succeeds.
    (text.strip_suffix(rest).unwrap_or_default(), rest)
}

fn push<'a>(runs: &mut Vec<(&'a str, Token)>, text: &'a str, token: Token) {
    if !text.is_empty() {
        runs.push((text, token));
    }
}

#[cfg(test)]
mod tests;
