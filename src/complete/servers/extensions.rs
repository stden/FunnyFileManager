//! Purpose: The Extensions field's suggestions — every extension the catalog
//! or the draft knows and the field does not list yet, with its languages.

use std::collections::{BTreeMap, BTreeSet};

use super::{candidate, offerable, Vocabulary};
use crate::complete::matcher;
use crate::config::{Extensions, LspSettings};
use crate::models::Candidate;

/// The extensions the rest of the field already lists, in lower case and
/// without their dots. The part of the current word after the caret is not
/// one of them.
pub(super) fn extensions_in(head: &str, after: &str) -> BTreeSet<String> {
    let rest = after.trim_start_matches(|ch: char| !Extensions::is_separator(ch));
    head.split(Extensions::is_separator)
        .chain(rest.split(Extensions::is_separator))
        .map(|word| word.strip_prefix('.').unwrap_or(word).to_ascii_lowercase())
        .filter(|word| !word.is_empty())
        .collect()
}

/// Every extension the catalog or the draft knows, except those the field
/// already lists, each with the languages that claim it.
pub(super) fn extensions(
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
