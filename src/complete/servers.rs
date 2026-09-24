//! Purpose: What the Language servers form offers while a command, arguments
//! or extensions are typed, and how an accepted suggestion lands in the field.
//!
//! Pure: every source arrives in a [`Vocabulary`] — the catalog, the draft and
//! the program names the probe found — so nothing here reads `PATH` or the
//! disk, and a keystroke costs a few string comparisons. The known values come
//! from the catalog and the draft alone: FFM talks to servers over stdio, so a
//! transport switch such as `--socket=` is never offered.

use std::collections::{BTreeMap, BTreeSet};

use super::{execs, matcher};
use crate::config::{quote, Arguments, Extensions, InstallRecipe, LspSettings};
use crate::lineedit::LineEditor;
use crate::models::{Candidate, CandidateKind};

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

/// The catalog's commands and npm binaries, then the commands in use, then
/// `PATH` programs starting with the token.
fn commands(word: &str, vocabulary: &Vocabulary<'_>) -> Vec<Candidate> {
    let catalog = catalog_commands(vocabulary.catalog);
    let known: BTreeSet<String> = catalog
        .iter()
        .cloned()
        .chain(
            vocabulary
                .draft
                .0
                .values()
                .map(|server| server.command.clone()),
        )
        .filter(|command| offerable(command))
        .collect();
    let mut rows: Vec<Candidate> = matcher::rank(word, known.iter().cloned().collect())
        .into_iter()
        .map(|name| {
            let hint = command_hint(&name, &catalog, vocabulary.programs);
            candidate(name, hint)
        })
        .collect();
    rows.extend(path_programs(word, &known, vocabulary.programs));
    rows
}

/// Where a known command comes from, and whether it is on `PATH` too.
fn command_hint(name: &str, catalog: &BTreeSet<String>, programs: &[String]) -> String {
    let origin = if catalog.contains(name) {
        "catalog"
    } else {
        "in use"
    };
    if on_path(name, programs) {
        format!("{origin} · PATH")
    } else {
        origin.to_owned()
    }
}

fn catalog_commands(catalog: &LspSettings) -> BTreeSet<String> {
    catalog
        .0
        .values()
        .flat_map(|server| {
            let binary = match &server.install {
                Some(InstallRecipe::Npm { binary, .. }) => Some(binary.clone()),
                _ => None,
            };
            std::iter::once(server.command.clone()).chain(binary)
        })
        .collect()
}

/// Whether `name` is on `PATH` under the file name LSP lookup would use.
fn on_path(name: &str, programs: &[String]) -> bool {
    programs
        .binary_search(&crate::lsp::program_file(name))
        .is_ok()
}

/// The `PATH` programs starting with `word` that LSP lookup would start and
/// the known list does not already hold; none for a short `word`.
fn path_programs(word: &str, known: &BTreeSet<String>, programs: &[String]) -> Vec<Candidate> {
    if word.chars().count() < MIN_PATH_PREFIX {
        return Vec::new();
    }
    // A launchable name already carries the file name lookup would use, so
    // it matches a known command spelled either way.
    let listed = |name: &String| {
        known
            .iter()
            .any(|command| *name == crate::lsp::program_file(command))
    };
    let found = execs::matching_in(programs, word)
        .into_iter()
        .filter(|name| crate::lsp::launchable_name(name) && !listed(name))
        .collect();
    matcher::rank(word, found)
        .into_iter()
        .map(|name| candidate(name, "PATH".to_owned()))
        .collect()
}

/// Every argument the catalog or the draft uses.
fn arguments(word: &str, vocabulary: &Vocabulary<'_>) -> Vec<Candidate> {
    let catalog: BTreeSet<&String> = vocabulary
        .catalog
        .0
        .values()
        .flat_map(|server| &server.args)
        .collect();
    let pool: BTreeSet<String> = catalog
        .iter()
        .map(|argument| (*argument).clone())
        .chain(
            vocabulary
                .draft
                .0
                .values()
                .flat_map(|server| server.args.clone()),
        )
        .filter(|argument| offerable(argument))
        .collect();
    matcher::rank(word, pool.into_iter().collect())
        .into_iter()
        .map(|argument| {
            let hint = if catalog.contains(&argument) {
                "catalog"
            } else {
                "in use"
            };
            candidate(argument, hint.to_owned())
        })
        .collect()
}

/// The extensions the rest of the field already lists, in lower case and
/// without their dots. The part of the current word after the caret is not
/// one of them.
fn extensions_in(head: &str, after: &str) -> BTreeSet<String> {
    let rest = after.trim_start_matches(|ch: char| !Extensions::is_separator(ch));
    head.split(Extensions::is_separator)
        .chain(rest.split(Extensions::is_separator))
        .map(|word| word.strip_prefix('.').unwrap_or(word).to_ascii_lowercase())
        .filter(|word| !word.is_empty())
        .collect()
}

/// Every extension the catalog or the draft knows, except those the field
/// already lists, each with the languages that claim it.
fn extensions(
    word: &str,
    present: &BTreeSet<String>,
    vocabulary: &Vocabulary<'_>,
) -> Vec<Candidate> {
    let owners = owners(vocabulary.draft);
    let pool: BTreeSet<String> = [vocabulary.catalog, vocabulary.draft]
        .into_iter()
        .flat_map(|settings| settings.0.values())
        .flat_map(|server| server.extensions.iter())
        .map(|extension| extension.strip_prefix('.').unwrap_or(extension).to_owned())
        .filter(|extension| offerable(extension))
        .filter(|extension| !present.contains(&extension.to_ascii_lowercase()))
        .collect();
    matcher::rank(word, pool.into_iter().collect())
        .into_iter()
        .map(|extension| {
            let hint = owners
                .get(&extension.to_ascii_lowercase())
                .map_or_else(|| "catalog".to_owned(), |languages| languages.join(", "));
            candidate(format!(".{extension}"), hint)
        })
        .collect()
}

/// The languages claiming each extension, by the extension in lower case.
fn owners(draft: &LspSettings) -> BTreeMap<String, Vec<String>> {
    let mut owners: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (language, server) in &draft.0 {
        for extension in &server.extensions {
            let extension = extension.strip_prefix('.').unwrap_or(extension);
            let claimed = owners.entry(extension.to_ascii_lowercase()).or_default();
            if !claimed.contains(language) {
                claimed.push(language.clone());
            }
        }
    }
    owners
}

#[cfg(test)]
mod tests;
