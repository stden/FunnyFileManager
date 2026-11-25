//! Purpose: AI provider selection, access mode, and `ffm.yml` `ai:` configuration schema.

use std::collections::BTreeMap;
use std::path::PathBuf;

/// Whether AI requests may reach external HTTP endpoints.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AiAccess {
    #[default]
    Offline,
    Network,
}

/// Supported intelligence API backends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AiProvider {
    #[default]
    Anthropic,
    OpenAi,
    Codex,
}

impl AiProvider {
    pub const ALL: [Self; 3] = [Self::Anthropic, Self::OpenAi, Self::Codex];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Anthropic => "Anthropic",
            Self::OpenAi => "OpenAI",
            Self::Codex => "Codex",
        }
    }

    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAi => "openai",
            Self::Codex => "codex",
        }
    }

    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "anthropic" | "claude" => Some(Self::Anthropic),
            "openai" | "gpt" => Some(Self::OpenAi),
            "codex" => Some(Self::Codex),
            _ => None,
        }
    }

    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Anthropic => Self::OpenAi,
            Self::OpenAi => Self::Codex,
            Self::Codex => Self::Anthropic,
        }
    }

    #[must_use]
    pub const fn default_base_url(self) -> &'static str {
        match self {
            Self::Anthropic => "https://api.anthropic.com",
            Self::OpenAi | Self::Codex => "https://api.openai.com",
        }
    }

    #[must_use]
    pub const fn default_model(self) -> &'static str {
        match self {
            Self::Anthropic => "claude-sonnet-4-5",
            Self::OpenAi => "gpt-5",
            Self::Codex => "gpt-5.3-codex",
        }
    }
}

/// Per-provider overrides in `ffm.yml` (`ai.anthropic`, `ai.openai`, `ai.codex`).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ProviderSettings {
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub headers: BTreeMap<String, String>,
}

/// User `ai:` settings in `ffm.yml`. API keys remain in env vars only.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AiSettings {
    pub provider: Option<String>,
    pub anthropic: ProviderSettings,
    pub openai: ProviderSettings,
    pub codex: ProviderSettings,
    pub tls_verify: Option<bool>,
    pub ca_cert: Option<PathBuf>,
    pub system_prompt: Option<String>,
}

impl AiSettings {
    #[must_use]
    pub const fn for_provider(&self, provider: AiProvider) -> &ProviderSettings {
        match provider {
            AiProvider::Anthropic => &self.anthropic,
            AiProvider::OpenAi => &self.openai,
            AiProvider::Codex => &self.codex,
        }
    }

    #[must_use]
    pub fn initial_provider(&self) -> AiProvider {
        self.provider
            .as_deref()
            .and_then(AiProvider::parse)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests;
