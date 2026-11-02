//! Purpose: The address and token of the catalog, read from the environment and
//! checked before a request is made. Neither value is ever printed.

use std::fmt;

use reqwest::Url;
use zeroize::Zeroizing;

/// The variable holding the catalog's address; it must be `https://`.
pub const URL_VARIABLE: &str = "THINGS_URL";
/// The variable holding the access token.
pub const TOKEN_VARIABLE: &str = "THINGS_API_TOKEN";

/// Where the catalog is, and the token that proves who is asking.
#[derive(Clone)]
pub struct Config {
    base: String,
    token: Token,
}

/// The access token, wiped from memory when its last copy is dropped.
#[derive(Clone)]
pub struct Token(Zeroizing<String>);

impl Token {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(<redacted>)")
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Config { url: <redacted>, token: <redacted> }")
    }
}

/// Why the settings cannot be used. Each message names the variable at fault,
/// never its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// These variables are unset or blank.
    Missing(Vec<&'static str>),
    /// The address does not start with `https://`.
    NotHttps,
    /// The address carries a user name or a password.
    UserInfo,
    /// The address carries a query or a fragment, which would swallow the list's path.
    QueryOrFragment,
    /// The address does not parse.
    Address,
    /// The token holds a control character, a line break among them.
    Token,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(names) => write!(f, "Set {} to browse Things", names.join(" and ")),
            Self::NotHttps => write!(f, "{URL_VARIABLE} must start with https://"),
            Self::UserInfo => write!(f, "{URL_VARIABLE} must not contain a user name or password"),
            Self::QueryOrFragment => write!(f, "{URL_VARIABLE} must have no query or fragment"),
            Self::Address => write!(f, "{URL_VARIABLE} is not a valid address"),
            Self::Token => write!(f, "{TOKEN_VARIABLE} must not contain control characters"),
        }
    }
}

impl Config {
    /// The settings the environment holds.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_values(
            std::env::var(URL_VARIABLE).ok(),
            std::env::var(TOKEN_VARIABLE).ok(),
        )
    }

    /// The settings two values make: both present, the address `https://` with
    /// no user information, query or fragment, the token free of control
    /// characters.
    pub fn from_values(url: Option<String>, token: Option<String>) -> Result<Self, ConfigError> {
        match (non_blank(url), non_blank(token)) {
            (Some(url), Some(token)) => Ok(Self {
                base: base_address(&url)?,
                token: access_token(token)?,
            }),
            (url, token) => Err(ConfigError::Missing(absent(url.is_none(), token.is_none()))),
        }
    }

    /// The address of the machine list.
    #[must_use]
    pub fn machines_url(&self) -> String {
        format!("{}/api/v1/machines", self.base)
    }

    #[must_use]
    pub const fn token(&self) -> &Token {
        &self.token
    }
}

fn non_blank(text: Option<String>) -> Option<String> {
    text.filter(|text| !text.trim().is_empty())
}

fn absent(url_missing: bool, token_missing: bool) -> Vec<&'static str> {
    [(url_missing, URL_VARIABLE), (token_missing, TOKEN_VARIABLE)]
        .into_iter()
        .filter_map(|(missing, name)| missing.then_some(name))
        .collect()
}

/// `text` as the catalog's address: trimmed of blanks and trailing slashes, an
/// `https` URL with no user information, query or fragment.
fn base_address(text: &str) -> Result<String, ConfigError> {
    let base = text.trim().trim_end_matches('/');
    let url = Url::parse(base).map_err(|_url_error| ConfigError::Address)?;
    if url.scheme() != "https" {
        return Err(ConfigError::NotHttps);
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(ConfigError::UserInfo);
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(ConfigError::QueryOrFragment);
    }
    Ok(base.to_owned())
}

/// `text` as the access token: trimmed, and free of control characters, which
/// no header value can hold. The buffer it came in is wiped on return.
fn access_token(text: String) -> Result<Token, ConfigError> {
    let secret = Zeroizing::new(text);
    let trimmed = secret.trim();
    if trimmed.chars().any(char::is_control) {
        return Err(ConfigError::Token);
    }
    Ok(Token(Zeroizing::new(trimmed.to_owned())))
}
