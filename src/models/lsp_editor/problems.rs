//! Purpose: What is wrong with the server table the dialog holds, and which of
//! it stops a save.
//!
//! Only a conflict the draft introduced blocks F2. Everything else was loaded
//! from the file: a save writes only the fields the draft changed, and those
//! passed the form, so refusing it would only keep the user from fixing the
//! rest.

use std::collections::{BTreeMap, BTreeSet};

use crate::config::{
    patch::OwnedField, Enablement, Extension, LanguageId, LspSettings, ServerSettings,
};

mod problem;

pub use problem::Problem;

/// Whether a problem stops F2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Impact {
    Blocking,
    Warning,
}

/// Every problem in `draft`, each marked blocking when the draft introduced
/// it: an extension both conflicting servers did not already claim in `baseline`.
#[must_use]
pub fn problems(draft: &LspSettings, baseline: &LspSettings) -> Vec<(Problem, Impact)> {
    let mut found: Vec<(Problem, Impact)> = duplicates(draft)
        .into_iter()
        .map(|problem| {
            let impact = if duplicate_was_loaded(&problem, baseline) {
                Impact::Warning
            } else {
                Impact::Blocking
            };
            (problem, impact)
        })
        .collect();
    for (language, server) in &draft.0 {
        let language = LanguageId::from_key(language);
        found.extend(
            server_problems(&language, server)
                .into_iter()
                .map(|problem| (problem, Impact::Warning)),
        );
    }
    found
}

/// A loaded conflict stays a warning when an earlier claimant is removed.
fn duplicate_was_loaded(problem: &Problem, baseline: &LspSettings) -> bool {
    let Problem::DuplicateExtension {
        extension,
        first,
        second,
    } = problem
    else {
        return false;
    };
    [first, second].into_iter().all(|language| {
        baseline.0.get(language.as_str()).is_some_and(|server| {
            server.enabled == Enablement::Enabled
                && server
                    .extensions
                    .iter()
                    .any(|claim| claim.eq_ignore_ascii_case(extension))
        })
    })
}

/// Each extension claimed by a later enabled server as well as an earlier
/// one, compared as `for_path` compares: ignoring ASCII case.
fn duplicates(settings: &LspSettings) -> Vec<Problem> {
    let mut first_claim: BTreeMap<String, &str> = BTreeMap::new();
    let mut found = Vec::new();
    for (language, server) in &settings.0 {
        if server.enabled != Enablement::Enabled {
            continue;
        }
        let own: BTreeSet<String> = server
            .extensions
            .iter()
            .map(|extension| extension.to_ascii_lowercase())
            .collect();
        for extension in own {
            match first_claim.get(extension.as_str()) {
                Some(first) => found.push(Problem::DuplicateExtension {
                    first: LanguageId::from_key(first),
                    second: LanguageId::from_key(language),
                    extension,
                }),
                None => {
                    first_claim.insert(extension, language);
                }
            }
        }
    }
    found
}

/// What is wrong with one server on its own.
fn server_problems(language: &LanguageId, server: &ServerSettings) -> Vec<Problem> {
    let control = |text: &str| text.chars().any(char::is_control);
    let mut found = Vec::new();
    let controlled = [
        (OwnedField::Command, control(&server.command)),
        (OwnedField::Args, server.args.iter().any(|arg| control(arg))),
        (
            OwnedField::Extensions,
            server.extensions.iter().any(|extension| control(extension)),
        ),
    ];
    for (field, holds) in controlled {
        if holds {
            found.push(Problem::ControlCharacter {
                language: language.clone(),
                field,
            });
        }
    }
    if server.command.trim().is_empty() {
        found.push(Problem::EmptyCommand {
            language: language.clone(),
        });
    }
    if server.extensions.is_empty() {
        found.push(Problem::NoExtensions {
            language: language.clone(),
        });
    }
    found.extend(invalid_extensions(language, server));
    found
}

/// The stored extensions that can never match: one with its dot, or one
/// that is not the text after the last dot of any name.
fn invalid_extensions<'a>(
    language: &'a LanguageId,
    server: &'a ServerSettings,
) -> impl Iterator<Item = Problem> + 'a {
    server
        .extensions
        .iter()
        .filter(|extension| !extension.chars().any(char::is_control))
        .filter(|extension| extension.starts_with('.') || Extension::parse(extension).is_err())
        .map(|extension| Problem::InvalidExtension {
            language: language.clone(),
            extension: extension.clone(),
        })
}

#[cfg(test)]
mod tests;
