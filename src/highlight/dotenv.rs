//! Dotenv assignment highlighting, independent of shell execution and Colorer.
//!
//! Values are text. Quotes can span lines; hashes inside quotes remain text,
//! and references are highlighted without expanding or reading their values.

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
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            push(runs, &line[..line.len() - trimmed.len()], Token::Normal);
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
        while let Some(ch) = self.line[self.at..].chars().next() {
            if ch == '\\' {
                self.at += ch.len_utf8();
                self.at += self.line[self.at..]
                    .chars()
                    .next()
                    .map_or(0, char::len_utf8);
            } else if matches!(self.state, State::Unquoted) && ch == '#' {
                self.emit(self.line.len() - self.at, Token::Comment);
                return self.runs;
            } else if !self.emit_special(ch) {
                self.advance_quote(ch);
                self.at += ch.len_utf8();
            }
        }
        push(&mut self.runs, &self.line[self.start..], Token::Str);
        self.runs
    }

    fn emit_special(&mut self, ch: char) -> bool {
        let rest = &self.line[self.at..];
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
        push(&mut self.runs, &self.line[self.start..self.at], Token::Str);
        self.runs
            .push((&self.line[self.at..self.at + length], token));
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
    let equals = line.find('=')?;
    let prefix = &line[..equals];
    let declaration = prefix.trim_start();
    let (export, key) = declaration
        .strip_prefix("export")
        .filter(|tail| tail.starts_with(char::is_whitespace))
        .map_or_else(
            || (None, declaration.trim_end()),
            |tail| (Some(prefix.len() - declaration.len()), tail.trim()),
        );
    if !valid_name(key) {
        return None;
    }

    let key_end = prefix.trim_end().len();
    let key_start = key_end - key.len();
    if let Some(export_start) = export {
        push(runs, &line[..export_start], Token::Normal);
        runs.push((&line[export_start..export_start + 6], Token::Keyword));
        push(runs, &line[export_start + 6..key_start], Token::Normal);
    } else {
        push(runs, &line[..key_start], Token::Normal);
    }
    runs.push((&line[key_start..key_end], Token::Type));
    push(runs, &line[key_end..equals], Token::Normal);
    runs.push((&line[equals..=equals], Token::Punctuation));
    let value_start = line.len() - line[equals + 1..].trim_start().len();
    push(runs, &line[equals + 1..value_start], Token::Normal);
    Some(value_start)
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
        let close = braced.find('}')?;
        let expression = &braced[..close];
        let name_length = run_length(expression, |ch| ch.is_ascii_alphanumeric() || ch == '_');
        return valid_name(&expression[..name_length]).then_some(close + 3);
    }
    let length = run_length(tail, |ch| ch.is_ascii_alphanumeric() || ch == '_');
    valid_name(&tail[..length]).then_some(length + 1)
}

fn push<'a>(runs: &mut Vec<(&'a str, Token)>, text: &'a str, token: Token) {
    if !text.is_empty() {
        runs.push((text, token));
    }
}

#[cfg(test)]
mod tests;
