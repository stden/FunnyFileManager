//! Purpose: AI provider secrets, resolved endpoint state, and YAML loading.

mod resolve;

use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use zeroize::Zeroizing;

use crate::values::ai::{AiError, AiProvider, AiSettings};
pub use resolve::resolve_with;

#[derive(serde::Deserialize)]
struct YamlDocument {
    #[serde(default)]
    ai: AiSettings,
}

/// API secret wiped on drop and always redacted in `Debug`.
#[derive(Clone)]
pub struct ApiSecret(Zeroizing<String>);

impl ApiSecret {
    pub(super) fn new(raw: String) -> Self {
        Self(Zeroizing::new(raw))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[cfg(test)]
    pub(super) fn try_new_for_test(raw: &str) -> Self {
        Self(Zeroizing::new(raw.to_owned()))
    }
}

impl fmt::Debug for ApiSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiSecret(<redacted>)")
    }
}

/// How the provider authenticates over HTTP.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthHeader {
    AnthropicKey,
    Bearer,
}

/// Validated runtime settings for one AI request.
#[derive(Clone)]
pub struct ResolvedProvider {
    pub provider: AiProvider,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub auth: AuthHeader,
    pub secret: ApiSecret,
    pub headers: BTreeMap<String, String>,
    pub tls_verify: bool,
    pub ca_cert: Option<PathBuf>,
    pub system_prompt: Option<String>,
}

impl fmt::Debug for ResolvedProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResolvedProvider")
            .field("provider", &self.provider)
            .field("model", &self.model)
            .field("secret", &"<redacted>")
            .finish_non_exhaustive()
    }
}

/// Load the optional `ai:` section from `ffm.yml`.
pub fn load_settings(path: &Path) -> io::Result<AiSettings> {
    let Some(stored) = crate::config::document::read_yaml(path)? else {
        return Ok(AiSettings::default());
    };
    let doc: YamlDocument = stored.parse()?;
    Ok(doc.ai)
}

/// Load default user `ai:` settings from `ffm.yml`, falling back to defaults.
#[must_use]
pub fn load_default_settings() -> AiSettings {
    let Some(path) = crate::config::dir().map(|dir| dir.join(crate::config::document::FILE_NAME))
    else {
        return AiSettings::default();
    };
    load_settings(&path).unwrap_or_default()
}

/// Resolve provider endpoint, model, TLS, and API key from `settings` and `std::env`.
pub fn resolve(provider: AiProvider, settings: &AiSettings) -> Result<ResolvedProvider, AiError> {
    resolve_with(provider, settings, |name| std::env::var(name).ok())
}
