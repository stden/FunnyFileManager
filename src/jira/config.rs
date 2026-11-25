//! Purpose: Resolve Jira base URL, TLS policy, JQL, and env token without leaking credentials.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use reqwest::Url;
use zeroize::Zeroizing;

use crate::values::jira::{JiraError, JiraSettings, TOKEN_VARIABLES};
use crate::values::{non_blank, parse_tls_verify};

#[derive(serde::Deserialize)]
struct YamlDocument {
    #[serde(default)]
    jira: JiraSettings,
}

/// Jira bearer personal access token wiped on drop and redacted from logs.
#[derive(Clone)]
pub struct Token(Zeroizing<String>);

impl Token {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[cfg(test)]
    pub(super) fn try_new_for_test(raw: &str) -> Self {
        Self(Zeroizing::new(raw.to_owned()))
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(<redacted>)")
    }
}

/// Validated runtime settings for communicating with Jira.
#[derive(Clone)]
pub struct RuntimeConfig {
    pub base_url: String,
    pub token: Token,
    pub tls_verify: bool,
    pub ca_cert: Option<PathBuf>,
    pub page_size: u16,
}

impl fmt::Debug for RuntimeConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeConfig")
            .field("base_url", &"<redacted>")
            .field("token", &"<redacted>")
            .field("tls_verify", &self.tls_verify)
            .field("page_size", &self.page_size)
            .finish_non_exhaustive()
    }
}

/// Load the optional `jira:` section from a YAML file (`ffm.yml`).
pub fn load_settings(path: &Path) -> io::Result<JiraSettings> {
    let Some(stored) = crate::config::document::read_yaml(path)? else {
        return Ok(JiraSettings::default());
    };
    let doc: YamlDocument = stored.parse()?;
    Ok(doc.jira)
}

/// Load the default user `jira:` settings from `ffm.yml`, falling back to defaults.
#[must_use]
pub fn load_default_settings() -> JiraSettings {
    let Some(path) = crate::config::dir().map(|dir| dir.join(crate::config::document::FILE_NAME))
    else {
        return JiraSettings::default();
    };
    load_settings(&path).unwrap_or_default()
}

/// Resolve runtime configuration from user settings and the process environment.
pub fn resolve_config(settings: &JiraSettings) -> Result<RuntimeConfig, JiraError> {
    resolve_config_with(settings, |name| std::env::var(name).ok())
}

/// Resolve runtime configuration using a caller-supplied environment lookup.
pub fn resolve_config_with(
    settings: &JiraSettings,
    env: impl Fn(&str) -> Option<String>,
) -> Result<RuntimeConfig, JiraError> {
    let raw_url = non_blank(env("JIRA_URL")).or_else(|| non_blank(settings.url.clone()));
    let Some(raw_url) = raw_url else {
        return Err(JiraError::Config(
            "Set JIRA_URL or jira.url in ffm.yml to browse Jira".to_owned(),
        ));
    };
    let base_url = validate_url(&raw_url)?;
    let token = resolve_token(&env)?;
    let tls_verify = parse_tls_verify(env("JIRA_TLS_VERIFY"), settings.tls_verify);
    let ca_cert = non_blank(env("JIRA_CA_CERT"))
        .map(PathBuf::from)
        .or_else(|| settings.ca_cert.clone());
    let page_size = settings.page_size.unwrap_or(50).clamp(1, 100);
    Ok(RuntimeConfig {
        base_url,
        token,
        tls_verify,
        ca_cert,
        page_size,
    })
}

/// Validate a Jira server URL (`https://`, no userinfo, no query or fragment).
pub fn validate_url(raw: &str) -> Result<String, JiraError> {
    let trimmed = raw.trim();
    let Ok(parsed) = Url::parse(trimmed) else {
        return Err(JiraError::Config(
            "JIRA_URL / jira.url is not a valid URL".to_owned(),
        ));
    };
    if parsed.scheme() != "https" {
        return Err(JiraError::Config(
            "Jira URL must start with https://".to_owned(),
        ));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(JiraError::Config(
            "Jira URL must not contain credentials".to_owned(),
        ));
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err(JiraError::Config(
            "Jira URL must not include a query or fragment".to_owned(),
        ));
    }
    Ok(trimmed.trim_end_matches('/').to_owned())
}

fn resolve_token(env: &impl Fn(&str) -> Option<String>) -> Result<Token, JiraError> {
    for var in TOKEN_VARIABLES {
        if let Some(value) = non_blank(env(var)) {
            if value.chars().any(char::is_control) {
                return Err(JiraError::Config(format!(
                    "{var} must not contain control characters"
                )));
            }
            return Ok(Token(Zeroizing::new(value)));
        }
    }
    Err(JiraError::Config(format!(
        "Set one of {} to browse Jira",
        TOKEN_VARIABLES.join(", ")
    )))
}
