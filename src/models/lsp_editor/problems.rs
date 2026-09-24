//! Purpose: What is wrong with the server table the dialog holds, and which of
//! it stops a save.
//!
//! Only a conflict the draft introduced blocks F2. Everything else was loaded
//! from the file: a save writes only the fields the draft changed, and those
//! passed the form, so refusing it would only keep the user from fixing the
//! rest.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use crate::config::{
    document, patch::OwnedField, Enablement, Extension, LanguageId, LspSettings, ServerSettings,
    LSP_KEY,
};

/// Something about one or two servers that keeps them from working.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// Two enabled servers claim one extension. The one earlier in the
    /// table starts for it, and the other never does.
    DuplicateExtension {
        extension: String,
        first: LanguageId,
        second: LanguageId,
    },
    /// An extension no file name can have.
    InvalidExtension {
        language: LanguageId,
        extension: String,
    },
    EmptyCommand {
        language: LanguageId,
    },
    NoExtensions {
        language: LanguageId,
    },
    /// A value the one-line form cannot show.
    ControlCharacter {
        language: LanguageId,
        field: OwnedField,
    },
}

impl Problem {
    /// Whether the problem concerns `language`.
    #[must_use]
    pub fn involves(&self, language: &LanguageId) -> bool {
        match self {
            Self::DuplicateExtension { first, second, .. } => {
                first == language || second == language
            }
            Self::InvalidExtension { language: own, .. }
            | Self::EmptyCommand { language: own }
            | Self::NoExtensions { language: own }
            | Self::ControlCharacter { language: own, .. } => own == language,
        }
    }

    /// The server a refused save selects: the first one involved.
    #[must_use]
    pub const fn language(&self) -> &LanguageId {
        match self {
            Self::DuplicateExtension { first, .. } => first,
            Self::InvalidExtension { language, .. }
            | Self::EmptyCommand { language }
            | Self::NoExtensions { language }
            | Self::ControlCharacter { language, .. } => language,
        }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateExtension {
                extension,
                first,
                second,
            } => write!(
                f,
                ".{extension} is claimed by {first} and {second} (both enabled)"
            ),
            // The form reads a stored extension as `Extension::parse` does, so
            // this names what OK stores only when OK would store it.
            Self::InvalidExtension { extension, .. } => match Extension::parse(extension) {
                Ok(stored) => write!(
                    f,
                    "{extension} is stored with its dot and never matches; OK in the form stores {stored}"
                ),
                Err(error) => write!(f, "{error}"),
            },
            Self::EmptyCommand { .. } => f.write_str("no command — this server never starts."),
            Self::NoExtensions { .. } => f.write_str("no extensions — this server never starts."),
            Self::ControlCharacter { language, field } => write!(
                f,
                "{LSP_KEY}.{language}.{} holds a control character; fix {} by hand, \
                 or press Del to reset a catalog server.",
                field.key(),
                document::FILE_NAME
            ),
        }
    }
}

/// Whether a problem stops F2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Impact {
    Blocking,
    Warning,
}

/// Every problem in `draft`, each marked blocking when the draft introduced
/// it: a claimed extension the `baseline` did not have claimed twice.
#[must_use]
pub fn problems(draft: &LspSettings, baseline: &LspSettings) -> Vec<(Problem, Impact)> {
    let loaded = duplicates(baseline);
    let mut found: Vec<(Problem, Impact)> = duplicates(draft)
        .into_iter()
        .map(|problem| {
            let impact = if loaded.contains(&problem) {
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
