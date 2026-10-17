//! Dotenv assignment highlighting, independent of shell execution and Colorer.
//!
//! Values are text. Quotes can span lines; hashes inside quotes remain text,
//! and references are highlighted without expanding or reading their values.

use std::ops::Range;

use super::{run_length, Token};

mod names;

use names::{assignment, reference_length};

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
            if ch == '\\' && !matches!(self.state, State::SingleQuoted) {
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
