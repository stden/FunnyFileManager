//! Purpose: The validated values of one language server — what the Language
//! servers dialog accepts as a language, a command, arguments and extensions.
//!
//! Each `parse` is the only constructor for user input, so a value that
//! reaches `ServerSettings` through the dialog has passed it. The messages of
//! [`ValueError`] are the dialog's refusals, written in one place.

use std::fmt;

use serde::{Deserialize, Serialize};

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

/// A server's arguments, edited on one line.
///
/// Whitespace outside quotes separates arguments. `"` opens or closes a quoted
/// run anywhere in an argument, as in a shell, so `--x="a b"` is `--x=a b`;
/// inside a run, `""` is one literal quote. There are no backslash escapes, so
/// a Windows path survives, and a control character is refused.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Arguments(Vec<String>);

impl Arguments {
    pub fn parse_line(line: &str) -> Result<Self, ValueError> {
        if line.chars().any(char::is_control) {
            return Err(ValueError::ArgumentControl);
        }
        let reader = LineReader::read(line);
        if reader.run == Run::Quoted {
            return Err(ValueError::QuoteNotClosed);
        }
        Ok(Self(reader.finish()))
    }

    /// The argument being typed at the end of `line`: the byte where it
    /// starts, and what it reads as so far. Between arguments it starts at the
    /// end of the line and reads as nothing.
    #[must_use]
    pub fn last(line: &str) -> (usize, String) {
        let reader = LineReader::read(line);
        match reader.current {
            Some(argument) => (reader.start, argument),
            None => (line.len(), String::new()),
        }
    }

    /// The line [`Self::parse_line`] reads back as these arguments: an
    /// argument that is empty or holds whitespace or a quote is quoted, with
    /// its quotes doubled.
    #[must_use]
    pub fn to_line(arguments: &[String]) -> String {
        arguments
            .iter()
            .map(|argument| quote(argument))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl From<Arguments> for Vec<String> {
    fn from(arguments: Arguments) -> Self {
        arguments.0
    }
}

/// One argument as [`Arguments::to_line`] writes it.
#[must_use]
pub fn quote(argument: &str) -> String {
    let plain = !argument.is_empty() && !argument.chars().any(|ch| ch.is_whitespace() || ch == '"');
    if plain {
        argument.to_owned()
    } else {
        format!("\"{}\"", argument.replace('"', "\"\""))
    }
}

/// Whether the argument reader is inside a quoted run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Run {
    #[default]
    Plain,
    Quoted,
}

/// The state of the argument grammar between characters.
#[derive(Default)]
struct LineReader {
    arguments: Vec<String>,
    /// The argument being read, `None` between arguments.
    current: Option<String>,
    /// The byte where `current` started.
    start: usize,
    run: Run,
}

impl LineReader {
    /// Read `line` to its end, whatever it holds.
    fn read(line: &str) -> Self {
        let mut reader = Self::default();
        let mut chars = line.char_indices().peekable();
        while let Some((index, ch)) = chars.next() {
            let doubled = ch == '"'
                && reader.run == Run::Quoted
                && chars.next_if(|&(_, next)| next == '"').is_some();
            reader.take(index, ch, doubled);
        }
        reader
    }

    /// Read the character at byte `index`; `doubled` is a `""` inside a
    /// quoted run.
    fn take(&mut self, index: usize, ch: char, doubled: bool) {
        let separates = ch.is_whitespace() && self.run == Run::Plain;
        if self.current.is_none() && !separates {
            self.start = index;
        }
        match ch {
            '"' if doubled => self.current.get_or_insert_default().push('"'),
            '"' => {
                self.run = match self.run {
                    Run::Plain => Run::Quoted,
                    Run::Quoted => Run::Plain,
                };
                self.current.get_or_insert_default();
            }
            _ if separates => self.arguments.extend(self.current.take()),
            ch => self.current.get_or_insert_default().push(ch),
        }
    }

    fn finish(mut self) -> Vec<String> {
        self.arguments.extend(self.current.take());
        self.arguments
    }
}

/// One file extension, stored without its dot, as `ServerSettings` holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extension(String);

impl Extension {
    /// One leading `.` is dropped. What is left must be the text after the
    /// last dot of a file name, since that is all FFM compares.
    pub fn parse(text: &str) -> Result<Self, ValueError> {
        let text = text.strip_prefix('.').unwrap_or(text);
        if text.is_empty() {
            return Err(ValueError::ExtensionEmpty);
        }
        if text.chars().any(char::is_control) {
            return Err(ValueError::ExtensionControl);
        }
        if text
            .chars()
            .any(|ch| Extensions::is_separator(ch) || ch == '/' || ch == '\\')
        {
            return Err(ValueError::ExtensionSeparator);
        }
        if text.contains('.') {
            return Err(ValueError::ExtensionDot(text.to_owned()));
        }
        Ok(Self(text.to_owned()))
    }
}

/// The extension as stored: without its dot.
impl fmt::Display for Extension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A server's extensions, edited on one line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Extensions(Vec<Extension>);

impl Extensions {
    /// Separated by whitespace or `,`; the same extension twice, in any
    /// letter case, is refused, because FFM ignores case when it matches.
    pub fn parse_line(line: &str) -> Result<Self, ValueError> {
        let mut extensions: Vec<Extension> = Vec::new();
        for word in line
            .split(Self::is_separator)
            .filter(|word| !word.is_empty())
        {
            let extension = Extension::parse(word)?;
            if let Some(first) = extensions
                .iter()
                .find(|seen| seen.0.eq_ignore_ascii_case(&extension.0))
            {
                return Err(ValueError::ExtensionTwice(first.0.clone()));
            }
            extensions.push(extension);
        }
        Ok(Self(extensions))
    }

    /// Whether `ch` separates two extensions on the form's line.
    #[must_use]
    pub fn is_separator(ch: char) -> bool {
        ch.is_whitespace() || ch == ','
    }

    /// The line the form shows: each extension with one dot, whether or not
    /// the file stored it with one, so [`Self::parse_line`] reads back what
    /// the file holds and a stored `.rs` becomes `rs`.
    #[must_use]
    pub fn to_line(extensions: &[String]) -> String {
        extensions
            .iter()
            .map(|extension| format!(".{}", extension.strip_prefix('.').unwrap_or(extension)))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl From<Extensions> for Vec<String> {
    fn from(extensions: Extensions) -> Self {
        extensions
            .0
            .into_iter()
            .map(|extension| extension.0)
            .collect()
    }
}

#[cfg(test)]
mod tests;
