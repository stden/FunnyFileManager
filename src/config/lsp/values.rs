//! Purpose: The validated values of one language server — what the Language
//! servers dialog accepts as a language, a command, arguments and extensions.
//!
//! Each `parse` is the only constructor for user input, so a value that
//! reaches `ServerSettings` through the dialog has passed it. The messages of
//! [`ValueError`] are the dialog's refusals, written in one place.

use std::fmt;

use serde::{Deserialize, Serialize};

mod arguments;
mod extensions;

pub use arguments::{quote, Arguments};
pub use extensions::{Extension, Extensions};

/// Whether FFM starts a server. Serialized as a boolean in the settings file
/// (`enabled: true` / `false`), so the file format is unchanged.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "bool", into = "bool")]
pub enum Enablement {
    #[default]
    Enabled,
    Disabled,
}

impl Enablement {
    /// The other state — what Space does.
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Enabled => Self::Disabled,
            Self::Disabled => Self::Enabled,
        }
    }
}

impl From<bool> for Enablement {
    fn from(enabled: bool) -> Self {
        if enabled {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }
}

impl From<Enablement> for bool {
    fn from(enablement: Enablement) -> Self {
        enablement == Enablement::Enabled
    }
}

/// Why a typed value was refused. Its `Display` is the sentence the dialog
/// shows under the form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueError {
    LanguageEmpty,
    LanguageCharacters,
    /// A new server named after a language already in the table.
    LanguageTaken(String),
    CommandEmpty,
    CommandControl,
    CommandTilde,
    QuoteNotClosed,
    ArgumentControl,
    ExtensionEmpty,
    /// An extension with a dot inside, as typed without its leading dot.
    ExtensionDot(String),
    ExtensionSeparator,
    ExtensionControl,
    /// The first spelling of an extension listed twice.
    ExtensionTwice(String),
}

impl fmt::Display for ValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LanguageEmpty => f.write_str("Enter the language: its LSP languageId."),
            Self::LanguageCharacters => f.write_str("Use a-z, 0-9 and - _ . + (LSP languageId)."),
            Self::LanguageTaken(language) => {
                write!(f, "{language} already exists; edit it instead.")
            }
            Self::CommandEmpty => f.write_str("Enter a command."),
            Self::CommandControl => f.write_str("The command cannot contain control characters."),
            Self::CommandTilde => {
                f.write_str("No shell runs the server, so ~ is not expanded; write the full path.")
            }
            Self::QuoteNotClosed => f.write_str("A quote is not closed."),
            Self::ArgumentControl => f.write_str("An argument cannot contain control characters."),
            Self::ExtensionEmpty => f.write_str("An extension cannot be empty."),
            Self::ExtensionDot(extension) => write!(
                f,
                ".{extension} can never match: FFM compares the text after the last dot."
            ),
            Self::ExtensionSeparator => {
                f.write_str("Extensions cannot contain spaces, commas, / or \\.")
            }
            Self::ExtensionControl => {
                f.write_str("An extension cannot contain control characters.")
            }
            Self::ExtensionTwice(extension) => write!(f, ".{extension} is listed twice."),
        }
    }
}

impl std::error::Error for ValueError {}

/// The longest language name accepted for a new server.
const MAX_LANGUAGE: usize = 64;

/// A language server's key in the table: the LSP `languageId`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LanguageId(String);

impl LanguageId {
    /// A new language typed by the user: `[a-z0-9][a-z0-9_.+-]{0,63}`.
    pub fn parse(text: &str) -> Result<Self, ValueError> {
        let text = text.trim();
        let mut chars = text.chars();
        let Some(first) = chars.next() else {
            return Err(ValueError::LanguageEmpty);
        };
        let lower_or_digit = |ch: char| ch.is_ascii_lowercase() || ch.is_ascii_digit();
        let valid = lower_or_digit(first)
            && chars.all(|ch| lower_or_digit(ch) || "_.+-".contains(ch))
            && text.len() <= MAX_LANGUAGE;
        if valid {
            Ok(Self(text.to_owned()))
        } else {
            Err(ValueError::LanguageCharacters)
        }
    }

    /// A key already in the table, kept verbatim: renaming is not offered, so
    /// a key the file holds is never rewritten.
    #[must_use]
    pub fn from_key(key: &str) -> Self {
        Self(key.to_owned())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LanguageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The program that runs a server: a name looked up on `PATH`, or a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerCommand(String);

impl ServerCommand {
    /// Trimmed; not empty, no control character, and no leading `~`, which
    /// only a shell would expand.
    pub fn parse(text: &str) -> Result<Self, ValueError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(ValueError::CommandEmpty);
        }
        if text.chars().any(char::is_control) {
            return Err(ValueError::CommandControl);
        }
        if text.starts_with('~') {
            return Err(ValueError::CommandTilde);
        }
        Ok(Self(text.to_owned()))
    }
}

impl From<ServerCommand> for String {
    fn from(command: ServerCommand) -> Self {
        command.0
    }
}

#[cfg(test)]
mod tests;
