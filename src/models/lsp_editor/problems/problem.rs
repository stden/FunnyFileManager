//! Purpose: One thing wrong with one or two servers of the table: which
//! servers it involves, and the sentence the dialog shows for it.

use std::fmt;

use crate::config::{document, patch::OwnedField, Extension, LanguageId, LSP_KEY};

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
