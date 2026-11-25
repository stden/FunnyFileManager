//! Purpose: The API token, read from its variable, sent only as a bearer value, and never printed.

use crate::values::cloudflare_dns::{CloudflareError, TOKEN_VARIABLE};
use std::fmt;
use zeroize::Zeroizing;

/// The API token. Its `Debug` and `Display` print a redacted form only.
pub struct Token(Zeroizing<String>);

impl Token {
    /// The trimmed `value`. A missing or blank one is [`CloudflareError::MissingToken`].
    pub(crate) fn from_value(value: Option<String>) -> Result<Self, CloudflareError> {
        value
            .map(|text| text.trim().to_owned())
            .filter(|text| !text.is_empty())
            .map(|text| Self(Zeroizing::new(text)))
            .ok_or(CloudflareError::MissingToken)
    }

    pub(crate) fn from_environment() -> Result<Self, CloudflareError> {
        Self::from_value(std::env::var(TOKEN_VARIABLE).ok())
    }

    /// The value of the `Authorization` header.
    pub(crate) fn authorization(&self) -> Zeroizing<String> {
        Zeroizing::new(format!("Bearer {}", self.0.as_str()))
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(<redacted>)")
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}
