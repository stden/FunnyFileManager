//! Purpose: The Command field's suggestions — the catalog's commands and npm
//! binaries, the commands in use, then the `PATH` programs LSP lookup would
//! start.

use std::collections::BTreeSet;

use super::{candidate, offerable, Vocabulary, MIN_PATH_PREFIX};
use crate::complete::{execs, matcher};
use crate::config::{InstallRecipe, LspSettings};
use crate::models::Candidate;

/// The catalog's commands and npm binaries, then the commands in use, then
/// `PATH` programs starting with the token.
pub(super) fn commands(word: &str, vocabulary: &Vocabulary<'_>) -> Vec<Candidate> {
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
