//! Purpose: Far's 8.3 form of a file name — what a panel shows with "Show
//! long names" (`Ctrl+N`) off.
//!
//! Far reads the short name from the filesystem, and only FAT and NTFS keep
//! one. Here the panel computes it the way Windows would have, so the same
//! listing reads the same on every platform: spaces and leading dots go, the
//! rest is uppercased, a name that then fits eight-plus-three is done, and
//! anything longer keeps six letters, a tilde and a number that is unique
//! within the listing — `LONGNA~1.EXT`.

use std::collections::{HashMap, HashSet};

use super::DirEntry;

/// Eight-plus-three: what a name may keep before it is shortened.
const STEM_LIMIT: usize = 8;
const EXTENSION_LIMIT: usize = 3;
/// The punctuation an 8.3 name may carry. Every other non-alphanumeric
/// character — `+,;=[]`, and anything outside ASCII — becomes `_`, as it does
/// on Windows.
const KEPT_PUNCTUATION: &str = "!#$%&'()-@^_`{}~";

/// A name in 8.3 form. Only ever built from a listing, which is what makes
/// the `~N` part unique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortName(String);

impl ShortName {
    /// Wrap a name already in 8.3 form.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ShortName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The 8.3 forms of one listing, keyed by long name.
///
/// Built from the whole listing because the `~N` of a name depends on what
/// precedes it: `~1` goes to the first `longna*.ext` in listing order, `~2`
/// to the next. A renderer builds one per panel draw — one pass over the
/// names — and looks each row up, rather than paying that pass per row.
#[derive(Debug, Clone, Default)]
pub struct ShortNames {
    by_name: std::sync::Arc<HashMap<String, ShortName>>,
}

impl ShortNames {
    /// Short names for `names`, numbered in the order given.
    #[must_use]
    pub fn for_listing<'a>(names: impl IntoIterator<Item = &'a str>) -> Self {
        let bases: Vec<(&'a str, Basis)> = names
            .into_iter()
            .map(|name| (name, Basis::of(name)))
            .collect();
        let reserved: HashSet<String> = bases
            .iter()
            .filter(|(name, basis)| !basis.stem.is_empty() && basis.fits(name))
            .map(|(_, basis)| basis.join(&basis.stem))
            .collect();
        let mut taken = HashSet::with_capacity(bases.len());
        let mut counters = HashMap::new();
        let mut by_name = HashMap::with_capacity(bases.len());
        for (name, basis) in bases {
            let short = unique_short_name(name, &basis, &taken, &reserved, &mut counters);
            taken.insert(short.0.clone());
            by_name.insert(name.to_owned(), short);
        }
        Self {
            by_name: std::sync::Arc::new(by_name),
        }
    }

    /// Short names for a panel's entries, in panel order.
    #[must_use]
    pub fn for_entries(entries: &[DirEntry]) -> Self {
        Self::for_listing(entries.iter().map(|entry| entry.name.as_str()))
    }

    /// The short form of a listed long name; `None` for a name that was not
    /// in the listing.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ShortName> {
        self.by_name.get(name)
    }
}

/// The first short form of `name` that nothing in the listing has claimed or
/// reserved as its own 8.3 name.
fn unique_short_name(
    name: &str,
    basis: &Basis,
    taken: &HashSet<String>,
    reserved: &HashSet<String>,
    counters: &mut HashMap<(String, String), usize>,
) -> ShortName {
    // `..`, or a name that is nothing but dots and blanks: there is no stem
    // to shorten, and Windows shows such an entry as it is.
    if basis.stem.is_empty() {
        return ShortName::new(name);
    }
    if basis.fits(name) {
        let plain = basis.join(&basis.stem);
        if !taken.contains(&plain) {
            return ShortName::new(plain);
        }
    }
    // `Basis::numbered` uses at most the first `STEM_LIMIT - 2` characters of
    // `stem` plus `extension`, and `taken` only grows, so resuming from the
    // last tested `n` for this prefix avoids rescanning `1..n` on collisions.
    let key = (
        basis.stem.chars().take(STEM_LIMIT - 2).collect(),
        basis.extension.clone(),
    );
    let next = counters.entry(key).or_insert(1);
    loop {
        let candidate = basis.numbered(*next);
        *next += 1;
        if !taken.contains(&candidate) && !reserved.contains(&candidate) {
            return ShortName::new(candidate);
        }
    }
}

/// What is left of a name once Windows has cleaned it: the uppercased stem
/// with its spaces and dots gone, and up to three letters of extension.
struct Basis {
    stem: String,
    extension: String,
}

impl Basis {
    fn of(name: &str) -> Self {
        let trimmed: String = name
            .trim_start_matches('.')
            .chars()
            .filter(|c| *c != ' ')
            .collect();
        let (stem, extension) = trimmed.rsplit_once('.').unwrap_or((&trimmed, ""));
        Self {
            stem: stem.chars().filter(|c| *c != '.').map(short_char).collect(),
            extension: extension
                .chars()
                .take(EXTENSION_LIMIT)
                .map(short_char)
                .collect(),
        }
    }

    /// `STEM.EXT`, or `STEM` alone where there is no extension.
    fn join(&self, stem: &str) -> String {
        if self.extension.is_empty() {
            return stem.to_owned();
        }
        format!("{stem}.{}", self.extension)
    }

    /// Whether the long name was, but for case, already a short one — in
    /// which case Windows gives it no `~N`.
    fn fits(&self, name: &str) -> bool {
        self.stem.len() <= STEM_LIMIT && name.to_uppercase() == self.join(&self.stem)
    }

    /// `STEM~N.EXT`, the stem cut so that stem and number together make
    /// eight: six letters for one digit, five for two.
    fn numbered(&self, n: usize) -> String {
        let suffix = format!("~{n}");
        let room = STEM_LIMIT.saturating_sub(suffix.len());
        let stem: String = self.stem.chars().take(room).collect();
        self.join(&format!("{stem}{suffix}"))
    }
}

/// One character of a long name as an 8.3 name keeps it.
fn short_char(c: char) -> char {
    if c.is_ascii_alphanumeric() || KEPT_PUNCTUATION.contains(c) {
        c.to_ascii_uppercase()
    } else {
        '_'
    }
}

#[cfg(test)]
mod tests;
