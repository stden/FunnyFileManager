//! Purpose: Quoting-aware word splitting and escaping for shell command lines.
//!
//! Pure: no `App`, no filesystem, no I/O. Completion outside `cd` has to speak
//! the same quoting language as the `sh -c` / `cmd /C` that eventually runs the
//! line, or a directory called `My Documents` completes into something the
//! shell reads as two arguments. Which language is the [`PathSyntax`]: `sh`
//! escapes with a backslash and quotes with `'` or `"`; `cmd` has no backslash
//! escape — there it is the path separator — and knows only `"`.
//!
//! One layer only. This is not a shell parser: it resolves quotes and
//! backslashes so a *token* can be matched and spliced back, and it stops
//! there — no expansion, no substitution, no operators.

use super::PathSyntax;

/// `sh` metacharacters escaped wherever they appear in a token.
///
/// Whitespace and quotes end the word; the rest either redirect, group,
/// substitute or glob. Escaping them all means a completed name is passed to
/// the shell as the literal text the filesystem actually holds.
const ALWAYS_ESCAPED: &[char] = &[
    ' ', '\t', '\n', '"', '\'', '\\', '$', '`', '&', '|', ';', '<', '>', '(', ')', '*', '?', '[',
    ']', '{', '}', '!',
];

/// `sh` metacharacters that are only special at the *start* of a word.
///
/// `~` expands to a home directory and `#` opens a comment only in leading
/// position. Escaping them everywhere would be worse than useless: the path
/// completer hands back tokens like `~/Documents/`, and `\~/Documents` is a
/// literal directory named `~` that does not exist.
const LEADING_ONLY_ESCAPED: &[char] = &['~', '#'];

/// `cmd` metacharacters: a word holding one of these, or whitespace, is
/// double-quoted as a whole, since cmd cannot escape a single character. `;`,
/// `,` and `=` delimit arguments to cmd's own built-ins (`dir a,b` lists two
/// names).
const WINDOWS_QUOTED: &[char] = &['&', '|', '<', '>', '^', '(', ')', ';', ',', '='];

/// Quote context while scanning a command line.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Quote {
    None,
    /// Inside `'…'`: nothing is special, not even a backslash. `sh` only.
    Single,
    /// Inside `"…"`: a backslash still escapes under `sh`.
    Double,
}

impl Quote {
    /// The quote state after `ch`, or `None` when `ch` opens or closes nothing
    /// here. cmd knows no single quote, so under Windows syntax `'` is an
    /// ordinary character.
    const fn after(self, syntax: PathSyntax, ch: char) -> Option<Self> {
        match (self, ch) {
            (Self::None, '"') => Some(Self::Double),
            (Self::None, '\'') if matches!(syntax, PathSyntax::Posix) => Some(Self::Single),
            (Self::Single, '\'') | (Self::Double, '"') => Some(Self::None),
            _ => None,
        }
    }

    /// Whether `ch` escapes the character after it: a backslash outside
    /// single quotes under `sh`; never under cmd, where it separates paths.
    const fn escapes(self, syntax: PathSyntax, ch: char) -> bool {
        ch == '\\' && matches!(syntax, PathSyntax::Posix) && !matches!(self, Self::Single)
    }
}

/// Split `input` into everything before the last word and that word itself.
///
/// The split point is the last whitespace that is neither quoted nor escaped,
/// so `cat My\ Doc` yields the word `My\ Doc` — not `Doc`, which a plain
/// whitespace split would give and which would complete against the wrong text.
/// An unterminated quote is treated as running to the end of the input, since
/// that is exactly the state a user is in while typing one.
#[must_use]
pub fn last_word(syntax: PathSyntax, input: &str) -> (&str, &str) {
    let mut start = 0;
    let mut quote = Quote::None;
    let mut escaped = false;

    for (index, ch) in input.char_indices() {
        if escaped {
            escaped = false;
        } else if quote.escapes(syntax, ch) {
            escaped = true;
        } else if let Some(next) = quote.after(syntax, ch) {
            quote = next;
        } else if quote == Quote::None && ch.is_whitespace() {
            // The word restarts after the separator, whatever its width:
            // U+00A0 and U+3000 are whitespace and are not one byte.
            start = index + ch.len_utf8();
        }
    }

    input.split_at(start)
}

/// The literal text the shell would see for `word`: one layer of quotes and
/// backslashes removed.
///
/// Total by construction — a trailing lone backslash (mid-typing) simply
/// escapes nothing and disappears, rather than being an error the completer
/// would have to report.
#[must_use]
pub fn unescape(syntax: PathSyntax, word: &str) -> String {
    let mut out = String::with_capacity(word.len());
    let mut quote = Quote::None;
    let mut escaped = false;

    for ch in word.chars() {
        if escaped {
            out.push(ch);
            escaped = false;
        } else if quote.escapes(syntax, ch) {
            escaped = true;
        } else if let Some(next) = quote.after(syntax, ch) {
            quote = next;
        } else {
            out.push(ch);
        }
    }
    out
}

/// `text` protected so that splicing it into a command line yields exactly one
/// argument holding exactly `text`: backslash-escaped for `sh`, double-quoted
/// for cmd.
///
/// Round-trip invariant, asserted in the tests: `unescape(escape(s)) == s`.
#[must_use]
pub fn escape(syntax: PathSyntax, text: &str) -> String {
    match syntax {
        PathSyntax::Posix => backslash_escaped(text),
        PathSyntax::Windows => double_quoted(text),
    }
}

/// `text` with every `sh` metacharacter backslash-escaped.
///
/// Backslashes rather than quotes: a quoted candidate would have to be
/// re-opened every time the user keeps typing into it, while an escaped one
/// stays a valid prefix of itself. `/` is deliberately *not* escaped — path
/// separators are the point of completing a path.
fn backslash_escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for (index, ch) in text.char_indices() {
        let leading = index == 0;
        if ALWAYS_ESCAPED.contains(&ch) || (leading && LEADING_ONLY_ESCAPED.contains(&ch)) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// `text` double-quoted when cmd would otherwise split it or read a
/// metacharacter in it; a plain word passes as it is.
///
/// Trailing separators stay outside the quotes: the C runtime reads `\"` as
/// a literal quote, so `"My Docs\"` never closes, while `"My Docs"\` is
/// `My Docs\` to cmd and to every program it starts — and it stays a valid
/// prefix as the user types on into the directory.
fn double_quoted(text: &str) -> String {
    let needs_quotes = text
        .chars()
        .any(|ch| ch.is_whitespace() || WINDOWS_QUOTED.contains(&ch));
    if !needs_quotes {
        return text.to_owned();
    }
    let body = text.trim_end_matches(|ch| PathSyntax::Windows.is_separator(ch));
    let (body, separators) = text.split_at(body.len());
    format!("\"{body}\"{separators}")
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_windows;
