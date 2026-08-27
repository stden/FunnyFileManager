//! Purpose: Quoting-aware word splitting and escaping for shell command lines.
//!
//! Pure: no `App`, no filesystem, no I/O. Completion outside `cd` has to speak
//! the same quoting language as the `sh -c` / `cmd /C` that eventually runs the
//! line, or a directory called `My Documents` completes into something the
//! shell reads as two arguments.
//!
//! One layer only. This is not a shell parser: it resolves quotes and
//! backslashes so a *token* can be matched and spliced back, and it stops
//! there — no expansion, no substitution, no operators.

/// Metacharacters escaped wherever they appear in a token.
///
/// Whitespace and quotes end the word; the rest either redirect, group,
/// substitute or glob. Escaping them all means a completed name is passed to
/// the shell as the literal text the filesystem actually holds.
const ALWAYS_ESCAPED: &[char] = &[
    ' ', '\t', '\n', '"', '\'', '\\', '$', '`', '&', '|', ';', '<', '>', '(', ')', '*', '?', '[',
    ']', '{', '}', '!',
];

/// Metacharacters that are only special at the *start* of a word.
///
/// `~` expands to a home directory and `#` opens a comment only in leading
/// position. Escaping them everywhere would be worse than useless: the path
/// completer hands back tokens like `~/Documents/`, and `\~/Documents` is a
/// literal directory named `~` that does not exist.
const LEADING_ONLY_ESCAPED: &[char] = &['~', '#'];

/// Quote context while scanning a command line.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Quote {
    None,
    /// Inside `'…'`: nothing is special, not even a backslash.
    Single,
    /// Inside `"…"`: a backslash still escapes.
    Double,
}

/// Split `input` into everything before the last word and that word itself.
///
/// The split point is the last whitespace that is neither quoted nor escaped,
/// so `cat My\ Doc` yields the word `My\ Doc` — not `Doc`, which a plain
/// whitespace split would give and which would complete against the wrong text.
/// An unterminated quote is treated as running to the end of the input, since
/// that is exactly the state a user is in while typing one.
#[must_use]
pub fn last_word(input: &str) -> (&str, &str) {
    let mut start = 0;
    let mut quote = Quote::None;
    let mut escaped = false;

    for (index, ch) in input.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match quote {
            Quote::None => match ch {
                '\\' => escaped = true,
                '\'' => quote = Quote::Single,
                '"' => quote = Quote::Double,
                // The word restarts after the separator, whatever its width:
                // U+00A0 and U+3000 are whitespace and are not one byte.
                _ if ch.is_whitespace() => start = index + ch.len_utf8(),
                _ => {}
            },
            Quote::Single => {
                if ch == '\'' {
                    quote = Quote::None;
                }
            }
            Quote::Double => match ch {
                '\\' => escaped = true,
                '"' => quote = Quote::None,
                _ => {}
            },
        }
    }

    input.split_at(start)
}

/// The literal text a shell would see for `word`: one layer of quotes and
/// backslashes removed.
///
/// Total by construction — a trailing lone backslash (mid-typing) simply
/// escapes nothing and disappears, rather than being an error the completer
/// would have to report.
#[must_use]
pub fn unescape(word: &str) -> String {
    let mut out = String::with_capacity(word.len());
    let mut quote = Quote::None;
    let mut escaped = false;

    for ch in word.chars() {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        match quote {
            Quote::None => match ch {
                '\\' => escaped = true,
                '\'' => quote = Quote::Single,
                '"' => quote = Quote::Double,
                _ => out.push(ch),
            },
            Quote::Single => {
                if ch == '\'' {
                    quote = Quote::None;
                } else {
                    out.push(ch);
                }
            }
            Quote::Double => match ch {
                '\\' => escaped = true,
                '"' => quote = Quote::None,
                _ => out.push(ch),
            },
        }
    }
    out
}

/// `text` with every shell metacharacter backslash-escaped, so splicing it into
/// a command line yields exactly one argument holding exactly `text`.
///
/// Backslashes rather than quotes: a quoted candidate would have to be
/// re-opened every time the user keeps typing into it, while an escaped one
/// stays a valid prefix of itself. `/` is deliberately *not* escaped — path
/// separators are the point of completing a path.
///
/// Round-trip invariant, asserted in the tests: `unescape(escape(s)) == s`.
#[must_use]
pub fn escape(text: &str) -> String {
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

#[cfg(test)]
mod tests;
