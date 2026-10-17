//! Purpose: The Arguments field's suggestions — every argument the catalog or
//! the draft uses, and nothing else.

use std::collections::BTreeSet;

use super::{candidate, offerable, Vocabulary};
use crate::complete::matcher;
use crate::models::Candidate;

/// Every argument the catalog or the draft uses.
pub(super) fn arguments(word: &str, vocabulary: &Vocabulary<'_>) -> Vec<Candidate> {
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
