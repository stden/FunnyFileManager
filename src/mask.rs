//! Purpose: Far's file masks — `*.rs`, `*.c,*.h`, `*|*.tmp`.
//!
//! Used by the selection keys and by anything else that has to say "these
//! files, not those". Written here rather than pulled in as a crate because a
//! mask is a hundred lines of matching and the alternative is a dependency in
//! the core of a program whose rule is the standard library.
//!
//! Far's syntax, as far as it is worth having:
//!
//! - `*` stands for any run of characters, `?` for exactly one.
//! - Several masks are separated by `,` or `;` — `*.c,*.h` is both.
//! - Everything after `|` is an exclusion: `*|*.tmp` is everything but
//!   temporary files.
//! - Any one mask may be put in double quotes, which makes `,`, `;`, `|` and
//!   the blanks at its ends part of it: `"semi;colon"` names that one file.
//!
//! Matching ignores case, as Far's does. On a case-sensitive filesystem that
//! is a deliberate choice rather than an oversight: a mask is typed by a
//! person in a hurry, and `*.JPG` failing to find `photo.jpg` is never what
//! they meant.

mod equality;

/// A parsed mask: what it takes, and what it takes back.
#[derive(Debug, Clone)]
pub struct Mask {
    include: Vec<Pattern>,
    exclude: Vec<Pattern>,
    source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Pattern {
    raw: String,
    chars: Vec<char>,
}

impl Pattern {
    fn new(raw: String) -> Self {
        let chars = fold_chars(&raw);
        Self { raw, chars }
    }

    fn matches(&self, name: &str, lower_chars: &mut Option<Vec<char>>) -> bool {
        if name.is_ascii() && self.raw.is_ascii() {
            return glob_slice(
                self.raw.as_bytes(),
                name.as_bytes(),
                |b| b == b'*',
                |p, n| p == b'?' || p == n.to_ascii_lowercase(),
            );
        }
        let chars = lower_chars.get_or_insert_with(|| fold_chars(name));
        glob_slice(&self.chars, chars, |c| c == '*', |p, n| p == '?' || p == n)
    }
}

fn fold_chars(text: &str) -> Vec<char> {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|ch| if ch == 'ς' { 'σ' } else { ch })
        .collect()
}

impl Mask {
    /// Parse a mask line. An empty include list means everything, so `|*.tmp`
    /// reads as "all but the temporary files" rather than as "nothing".
    #[must_use]
    pub fn parse(line: &str) -> Self {
        let bar = unquoted(line).find(|&(_, ch)| ch == '|');
        // `at` is a character boundary and `|` one byte, so both halves exist.
        let (take, drop) = bar.map_or((line, ""), |(at, _)| {
            (
                line.get(..at).unwrap_or_default(),
                line.get(at + 1..).unwrap_or_default(),
            )
        });
        Self {
            include: patterns(take),
            exclude: patterns(drop),
            source: line.to_owned(),
        }
    }

    /// Whether `name` is one of the files this mask is about.
    #[must_use]
    pub fn matches(&self, name: &str) -> bool {
        // A line with nothing on either side of the bar is not "everything" —
        // it is a field the user cleared, and it selects nothing.
        if self.is_empty() {
            return false;
        }
        let mut lower_chars = None;
        if self
            .exclude
            .iter()
            .any(|p| p.matches(name, &mut lower_chars))
        {
            return false;
        }
        self.include.is_empty()
            || self
                .include
                .iter()
                .any(|p| p.matches(name, &mut lower_chars))
    }

    /// Whether this mask would let anything through at all. An empty line
    /// selects nothing rather than everything: the user who cleared the field
    /// did not mean "all files".
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.include.is_empty() && self.exclude.is_empty()
    }
}

/// Split a half of the line into lowercase patterns, dropping blanks.
///
/// Only separators outside quotes split. Each part is trimmed before its
/// quotes are removed, so the blanks inside them stay part of the name.
fn patterns(half: &str) -> Vec<Pattern> {
    // Separators are one byte at character boundaries, so every `get` succeeds.
    let mut parts = Vec::new();
    let mut start = 0;
    for (at, _) in unquoted(half).filter(|&(_, ch)| ch == ',' || ch == ';') {
        parts.push(half.get(start..at).unwrap_or_default());
        start = at + 1;
    }
    parts.push(half.get(start..).unwrap_or_default());
    parts
        .into_iter()
        .map(|part| part.trim().replace('"', ""))
        .filter(|part| !part.is_empty())
        .map(|part| Pattern::new(part.to_lowercase()))
        .collect()
}

/// The characters of `text` that stand outside double quotes, with their byte
/// offsets. The quotes themselves are left out, so a separator between them is
/// never seen and cannot split the mask.
fn unquoted(text: &str) -> impl Iterator<Item = (usize, char)> + '_ {
    let mut quoted = false;
    text.char_indices().filter(move |&(_, ch)| {
        if ch == '"' {
            quoted = !quoted;
        }
        !quoted && ch != '"'
    })
}

/// Match one pattern slice against one candidate slice with iterative
/// single-star backtracking.
fn glob_slice<T: Copy>(
    pattern: &[T],
    name: &[T],
    is_star: impl Fn(T) -> bool,
    matches_one: impl Fn(T, T) -> bool,
) -> bool {
    let (mut at_pattern, mut at_name) = (0, 0);
    let mut star: Option<usize> = None;
    let mut swallowed = 0;

    while let Some(&current) = name.get(at_name) {
        match pattern.get(at_pattern).copied() {
            Some(ch) if is_star(ch) => {
                star = Some(at_pattern);
                swallowed = at_name;
                at_pattern += 1;
            }
            Some(ch) if matches_one(ch, current) => {
                at_pattern += 1;
                at_name += 1;
            }
            _ => match star {
                Some(at) => {
                    at_pattern = at + 1;
                    swallowed += 1;
                    at_name = swallowed;
                }
                None => return false,
            },
        }
    }
    pattern
        .get(at_pattern..)
        .unwrap_or_default()
        .iter()
        .copied()
        .all(is_star)
}

#[cfg(test)]
mod tests;
