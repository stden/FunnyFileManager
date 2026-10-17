//! Purpose: A server's file extensions on the form's one line — each stored
//! without its dot, none listed twice.

use std::fmt;

use super::ValueError;

/// One file extension, stored without its dot, as `ServerSettings` holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extension(pub(super) String);

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
