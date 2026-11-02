//! Purpose: What the Language servers form offers while a command, arguments
//! or extensions are typed, and how an accepted suggestion lands in the field.
//!
//! Pure: every source arrives in a [`Vocabulary`] — the catalog, the draft and
//! the program names the probe found — so nothing here reads `PATH` or the
//! disk, and a keystroke costs a few string comparisons. The known values come
//! from the catalog and the draft alone: FFM talks to servers over stdio, so a
//! transport switch such as `--socket=` is never offered.

use crate::config::{quote, Arguments, Extensions, LspSettings};
use crate::lineedit::LineEditor;
use crate::models::{Candidate, CandidateKind};

mod arguments;
mod commands;
mod extensions;

use arguments::arguments;
use commands::commands;
use extensions::{extensions, extensions_in};

/// `PATH` programs join the Command list from this many typed characters:
/// one letter matches hundreds of them, which is a wall, not a choice.
pub const MIN_PATH_PREFIX: usize = 2;

/// Where the suggestions come from.
pub struct Vocabulary<'a> {
    /// The embedded catalog, `LspSettings::catalog()`.
    pub catalog: &'a LspSettings,
    /// What the dialog currently holds.
    pub draft: &'a LspSettings,
    /// The program names on `PATH`, sorted; empty while the scan runs.
    pub programs: &'a [String],
}

/// A form field that offers suggestions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletedField {
    Command,
    Arguments,
    Extensions,
}

/// The suggestions for the token before the caret of `line`, best first.
#[must_use]
pub fn candidates(
    field: CompletedField,
    line: &LineEditor,
    vocabulary: &Vocabulary<'_>,
) -> Vec<Candidate> {
    let token = Token::before(field, line.before_cursor());
    match field {
        CompletedField::Command => commands(&token.word, vocabulary),
        CompletedField::Arguments => arguments(&token.word, vocabulary),
        CompletedField::Extensions => {
            let present = extensions_in(token.head, line.after_cursor());
            extensions(&token.word, &present, vocabulary)
        }
    }
}

/// The token before the caret, spelled as a candidate would spell it — what
/// a suggestion equal to it would complete to nothing.
#[must_use]
pub fn typed(field: CompletedField, line: &LineEditor) -> String {
    let token = Token::before(field, line.before_cursor());
    match field {
        CompletedField::Extensions if !token.word.is_empty() => format!(".{}", token.word),
        _ => token.word,
    }
}

/// Put `text`, a candidate's text, in place of the token before the caret.
///
/// A command is taken as it is. An argument is quoted if it needs it, and an
/// argument or an extension is followed by one space unless the text after
/// the caret already starts with one.
pub fn accept(field: CompletedField, line: &mut LineEditor, text: &str) {
    let head = Token::before(field, line.before_cursor()).head.to_owned();
    let tail = line.after_cursor().to_owned();
    let space = if tail.starts_with(char::is_whitespace) {
        ""
    } else {
        " "
    };
    let inserted = match field {
        CompletedField::Command => text.to_owned(),
        CompletedField::Arguments => format!("{}{space}", quote(text)),
        CompletedField::Extensions => format!("{text}{space}"),
    };
    line.set_split(&format!("{head}{inserted}"), &tail);
}

/// The token being completed: the text before it, and the token as matched.
struct Token<'a> {
    head: &'a str,
    word: String,
}

impl<'a> Token<'a> {
    fn before(field: CompletedField, before: &'a str) -> Self {
        match field {
            CompletedField::Command => {
                let word = before.trim_start();
                Self {
                    head: before.strip_suffix(word).unwrap_or_default(),
                    word: word.to_owned(),
                }
            }
            CompletedField::Arguments => {
                let (start, word) = Arguments::last(before);
                Self {
                    head: split(before, start).0,
                    word,
                }
            }
            CompletedField::Extensions => {
                // After the separator's last byte: a no-break or ideographic
                // space is more than one byte long.
                let start = before
                    .char_indices()
                    .rev()
                    .find(|&(_, ch)| Extensions::is_separator(ch))
                    .map_or(0, |(at, ch)| at + ch.len_utf8());
                let (head, word) = split(before, start);
                Self {
                    head,
                    word: word.strip_prefix('.').unwrap_or(word).to_owned(),
                }
            }
        }
    }
}

/// `text` split at the token's first byte `start`, which the callers take
/// from a char boundary. Were it ever not one, the whole text stays the head
/// and the completion is appended rather than cutting a character in two.
fn split(text: &str, start: usize) -> (&str, &str) {
    text.split_at_checked(start).unwrap_or((text, ""))
}

/// A value from a settings file that can be put in a one-line field.
fn offerable(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_control)
}

fn candidate(text: String, hint: String) -> Candidate {
    Candidate {
        text,
        kind: CandidateKind::Word,
        hint: Some(hint),
    }
}

#[cfg(test)]
mod tests;
