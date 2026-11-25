//! Purpose: AI request, decoded response, and typed failure values.

use std::fmt;

use super::{AiProvider, AiSettings, ChatMessage, WorkspaceContext};

/// Successful assistant reply with optional extracted executable command and token counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiResponse {
    pub provider: AiProvider,
    pub model: String,
    pub text: String,
    pub command: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

/// Prompt execution request queued from the AI dialog to the worker thread.
#[derive(Debug, Clone)]
pub struct AiRequest {
    pub provider: AiProvider,
    pub messages: Vec<ChatMessage>,
    pub context: WorkspaceContext,
    pub settings: AiSettings,
}

/// Typed AI errors; API keys and gateway tokens are never formatted into messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiError {
    Disabled,
    MissingKey(String),
    Config(String),
    Network(String),
    Http(u16, String),
    InvalidResponse(String),
    Worker,
}

impl fmt::Display for AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => f.write_str("Unavailable in replay or isolated sessions"),
            Self::MissingKey(msg)
            | Self::Config(msg)
            | Self::Network(msg)
            | Self::InvalidResponse(msg) => f.write_str(msg),
            Self::Http(code, msg) => write!(f, "AI HTTP {code}: {msg}"),
            Self::Worker => f.write_str("AI worker stopped unexpectedly"),
        }
    }
}

#[cfg(test)]
mod tests;
