//! Purpose: Provider identities, token units and the token usage panel's request state.
use super::Viewer;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    OpenAi,
    Anthropic,
    OpenRouter,
}
impl Provider {
    pub const ALL: [Self; 3] = [Self::OpenAi, Self::Anthropic, Self::OpenRouter];
    pub const fn label(self) -> &'static str {
        match self {
            Self::OpenAi => "OpenAI",
            Self::Anthropic => "Anthropic",
            Self::OpenRouter => "OpenRouter",
        }
    }
    pub const fn key_variable(self) -> &'static str {
        match self {
            Self::OpenAi => "OPENAI_ADMIN_KEY",
            Self::Anthropic => "ANTHROPIC_ADMIN_KEY",
            Self::OpenRouter => "OPENROUTER_MANAGEMENT_KEY",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(transparent)]
pub struct Tokens(u64);
impl Tokens {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn count(self) -> u64 {
        self.0
    }
    pub fn add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UsageTotals {
    pub input: Tokens,
    pub output: Tokens,
    pub cached: Option<Tokens>,
    pub requests: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsageError {
    MissingKey,
    Disabled,
    AccessDenied,
    RateLimited,
    Network,
    InvalidResponse,
    Limit,
    Worker,
}
impl UsageError {
    pub const fn message(self) -> &'static str {
        match self {
            Self::Disabled => "Unavailable in replay or isolated sessions",
            Self::MissingKey => "Not configured",
            Self::AccessDenied => "Access denied: admin/management key required",
            Self::RateLimited => "Rate limited; retry later",
            Self::Network => "Network request failed",
            Self::InvalidResponse => "Unsupported provider response",
            Self::Limit => "Report exceeded its time, size or pagination limit",
            Self::Worker => "Usage worker failed",
        }
    }
}
#[derive(Clone, Debug)]
pub struct ProviderUsage {
    pub provider: Provider,
    pub result: Result<UsageTotals, UsageError>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsageState {
    Queued,
    Loading,
    Ready,
}
#[derive(Clone)]
pub struct TokenUsagePanel {
    pub(crate) viewer: Viewer,
    pub(crate) state: UsageState,
    pub(crate) generation: Arc<()>,
}
impl TokenUsagePanel {
    pub(crate) fn new() -> Self {
        Self {
            viewer: Viewer::report(
                "Token usage | F5 refresh | Esc close",
                vec!["Loading provider reports...".to_owned()],
            ),
            state: UsageState::Queued,
            generation: Arc::new(()),
        }
    }
}

#[derive(Clone, Copy, Default)]
pub enum UsageAccess {
    #[default]
    Offline,
    Network,
}
