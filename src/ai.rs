//! Purpose: Intelligence backend supporting Anthropic Messages, `OpenAI` Chat, and Codex Responses.

pub mod client;
pub mod config;
pub mod extract;
pub mod providers;

use crate::values::ai::{AiError, AiRequest, AiResponse, ChatMessage, WorkspaceContext};
use client::{AiTransport, HttpAiTransport};
use config::{resolve, ResolvedProvider};
use providers::{build_request, decode_response};

/// Execute one AI prompt request against the configured provider endpoint.
pub fn execute(request: &AiRequest) -> Result<AiResponse, AiError> {
    let resolved = resolve(request.provider, &request.settings)?;
    let mut transport = HttpAiTransport::from_resolved(&resolved)?;
    execute_with(
        &mut transport,
        &resolved,
        &request.messages,
        &request.context,
    )
}

/// Execute one AI prompt through `transport`.
pub fn execute_with(
    transport: &mut dyn AiTransport,
    resolved: &ResolvedProvider,
    messages: &[ChatMessage],
    context: &WorkspaceContext,
) -> Result<AiResponse, AiError> {
    let (path, body) = build_request(resolved, messages, context);
    let raw = transport.post(path, &body)?;
    decode_response(resolved.provider, &resolved.model, &raw)
}

/// Answer when network access is disabled (replays / tests).
pub const fn unavailable() -> Result<AiResponse, AiError> {
    Err(AiError::Disabled)
}

#[cfg(test)]
mod tests;
