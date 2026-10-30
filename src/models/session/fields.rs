//! Purpose: The checked values a stored session is made of — its name, host,
//! port, user and key file — and the one error every check reports in.
//!
//! Each newtype has one constructor that checks, and `serde` goes through
//! it, so a value typed in the dialog, loaded from `sessions.json` or read
//! from an imported file is refused by the same rule wherever it comes from.
//! Text is trimmed first; a surrounding blank is never part of a name.

use std::fmt;

use serde::{Deserialize, Serialize};

mod invalid;
mod port;

pub use invalid::{Field, InvalidField, Problem};
pub use port::Port;

/// The longest session name, in characters: room for the longest default
/// name, `user@host:port`, and then some.
pub const MAX_NAME_CHARS: usize = 512;
/// The longest host name DNS allows, in characters.
pub const MAX_HOST_CHARS: usize = 253;
/// The longest user name, in characters.
pub const MAX_USER_CHARS: usize = 128;
/// The longest key file path, in characters.
pub const MAX_KEY_FILE_CHARS: usize = 4096;

/// `text` trimmed, when it is not empty, not longer than `limit` characters
/// and free of control characters — the check every text value shares.
fn checked(field: Field, text: &str, limit: usize) -> Result<String, InvalidField> {
    let text = text.trim();
    let fail = |problem| Err(InvalidField::new(field, problem));
    if text.is_empty() {
        return fail(Problem::Empty);
    }
    if text.chars().count() > limit {
        return fail(Problem::TooLong { limit });
    }
    if text.chars().any(char::is_control) {
        return fail(Problem::ControlCharacter);
    }
    Ok(text.to_owned())
}

/// `text` with no leading `-`.
fn without_leading_dash(field: Field, text: String) -> Result<String, InvalidField> {
    if text.starts_with('-') {
        return Err(InvalidField::new(field, Problem::LeadingDash));
    }
    Ok(text)
}

/// The name a session is listed and looked up by. A newtype so a host or a
/// user name — which look alike, and are often equal — cannot be handed to
/// a lookup by mistake.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SessionName(String);

impl SessionName {
    pub fn new(text: &str) -> Result<Self, InvalidField> {
        checked(Field::Name, text, MAX_NAME_CHARS).map(Self)
    }

    /// `NetBox`'s default name: `user@host`, with `:port` when the port is
    /// not the protocol's own.
    #[must_use]
    pub fn default_for(
        user: Option<&UserName>,
        host: &HostName,
        port: Port,
        default_port: Port,
    ) -> Self {
        let login = user.map_or_else(|| host.as_str().to_owned(), |user| format!("{user}@{host}"));
        // Both parts passed their own checks and together stay far below
        // the name's limit, so the result needs no second check.
        if port == default_port {
            Self(login)
        } else {
            Self(format!("{login}:{port}"))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The host a session connects to: a DNS name or an IP address.
///
/// Letters, digits, `.`, `-`, `_` and the `:` of an IPv6 address, and no
/// leading `-` — nothing a shell, a URL or an `ssh` command line would read
/// as anything but a host.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct HostName(String);

impl HostName {
    pub fn new(text: &str) -> Result<Self, InvalidField> {
        let text = checked(Field::Host, text, MAX_HOST_CHARS)?;
        let allowed = |ch: char| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | ':');
        if let Some(bad) = text.chars().find(|ch| !allowed(*ch)) {
            return Err(InvalidField::new(Field::Host, Problem::Character(bad)));
        }
        without_leading_dash(Field::Host, text).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The login a session connects as. Any printable text but a leading `-`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct UserName(String);

impl UserName {
    pub fn new(text: &str) -> Result<Self, InvalidField> {
        let text = checked(Field::User, text, MAX_USER_CHARS)?;
        without_leading_dash(Field::User, text).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A private key file the session logs in with.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct KeyFile(String);

impl KeyFile {
    pub fn new(text: &str) -> Result<Self, InvalidField> {
        checked(Field::KeyFile, text, MAX_KEY_FILE_CHARS).map(Self)
    }
}

/// `Display` and the `serde` conversions every text newtype shares.
macro_rules! text_value {
    ($($name:ident),+) => {$(
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl TryFrom<String> for $name {
            type Error = InvalidField;

            fn try_from(text: String) -> Result<Self, Self::Error> {
                Self::new(&text)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    )+};
}

text_value!(SessionName, HostName, UserName, KeyFile);

#[cfg(test)]
mod tests;
