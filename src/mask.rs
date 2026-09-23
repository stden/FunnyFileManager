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

/// A parsed mask: what it takes, and what it takes back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mask {
    include: Vec<String>,
    exclude: Vec<String>,
}

impl Mask {
    /// Parse a mask line. An empty include list means everything, so `|*.tmp`
    /// reads as "all but the temporary files" rather than as "nothing".
    #[must_use]
    pub fn parse(line: &str) -> Self {
        let bar = unquoted(line).find(|&(_, ch)| ch == '|');
        let (take, drop) = bar.map_or((line, ""), |(at, _)| (&line[..at], &line[at + 1..]));
        Self {
            include: patterns(take),
            exclude: patterns(drop),
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
        let name = name.to_lowercase();
        let any = |patterns: &[String]| patterns.iter().any(|p| glob(p, &name));
        if any(&self.exclude) {
            return false;
        }
        self.include.is_empty() || any(&self.include)
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
fn patterns(half: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut start = 0;
    for (at, _) in unquoted(half).filter(|&(_, ch)| ch == ',' || ch == ';') {
        parts.push(&half[start..at]);
        start = at + 1;
    }
    parts.push(&half[start..]);
    parts
        .into_iter()
        .map(|part| part.trim().replace('"', ""))
        .filter(|part| !part.is_empty())
        .map(|part| part.to_lowercase())
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

/// Match one lowercase pattern against one lowercase name.
///
/// Iterative with one backtrack point rather than recursive: a pattern like
/// `*a*a*a*a*b` against a long name costs exponential time recursively, and a
/// file manager must not freeze because someone typed a silly mask.
fn glob(pattern: &str, name: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let name: Vec<char> = name.chars().collect();
    let (mut at_pattern, mut at_name) = (0, 0);
    // Where the last `*` was, and how much of the name it had swallowed —
    // the state a failed match rewinds to.
    let mut star: Option<usize> = None;
    let mut swallowed = 0;

    while at_name < name.len() {
        match pattern.get(at_pattern) {
            Some('*') => {
                star = Some(at_pattern);
                swallowed = at_name;
                at_pattern += 1;
            }
            Some(ch) if *ch == '?' || *ch == name[at_name] => {
                at_pattern += 1;
                at_name += 1;
            }
            _ => match star {
                // Give the `*` one more character and try again from there.
                Some(at) => {
                    at_pattern = at + 1;
                    swallowed += 1;
                    at_name = swallowed;
                }
                None => return false,
            },
        }
    }
    // Whatever is left of the pattern must be able to match nothing.
    pattern[at_pattern.min(pattern.len())..]
        .iter()
        .all(|ch| *ch == '*')
}

#[cfg(test)]
mod tests;
