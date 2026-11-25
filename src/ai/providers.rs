//! Purpose: Request builders and response decoders for Anthropic, `OpenAI`, and Codex APIs.

use serde_json::{json, Value};

use super::config::ResolvedProvider;
use super::extract::first_shell_command;
use crate::values::ai::{AiError, AiProvider, AiResponse, ChatMessage, WorkspaceContext};

/// Build the system instruction text describing FFM workspace state and user overrides.
#[must_use]
pub fn system_instructions(context: &WorkspaceContext, custom: Option<&str>) -> String {
    let mut parts = vec![
        "You are an assistant integrated inside FunnyFileManager (FFM), a dual-panel terminal file manager."
            .to_owned(),
        "Prefer concise answers and place any ready-to-run shell command in a ```bash or ```pwsh fenced block."
            .to_owned(),
        context.prompt_summary(),
    ];
    if let Some(extra) = custom.map(str::trim).filter(|s| !s.is_empty()) {
        parts.push(extra.to_owned());
    }
    parts.join("\n\n")
}

/// Build the HTTP endpoint path and JSON body for the selected provider.
#[must_use]
pub fn build_request(
    resolved: &ResolvedProvider,
    messages: &[ChatMessage],
    context: &WorkspaceContext,
) -> (&'static str, Value) {
    let system = system_instructions(context, resolved.system_prompt.as_deref());
    match resolved.provider {
        AiProvider::Anthropic => (
            "/v1/messages",
            json!({
                "model": resolved.model,
                "max_tokens": resolved.max_tokens,
                "system": system,
                "messages": api_messages(messages),
            }),
        ),
        AiProvider::OpenAi => (
            "/v1/chat/completions",
            openai_body(resolved, messages, &system),
        ),
        AiProvider::Codex => (
            "/v1/responses",
            json!({
                "model": resolved.model,
                "max_output_tokens": resolved.max_tokens,
                "instructions": system,
                "input": api_messages(messages),
            }),
        ),
    }
}

fn openai_body(resolved: &ResolvedProvider, messages: &[ChatMessage], system: &str) -> Value {
    let mut turns = vec![json!({"role": "system", "content": system})];
    turns.extend(api_messages(messages));
    json!({
        "model": resolved.model,
        "max_completion_tokens": resolved.max_tokens,
        "messages": turns,
    })
}

fn api_messages(messages: &[ChatMessage]) -> Vec<Value> {
    messages
        .iter()
        .map(|msg| json!({"role": msg.role.as_api_str(), "content": msg.text}))
        .collect()
}

/// Decode provider JSON into `AiResponse`.
pub fn decode_response(
    provider: AiProvider,
    fallback_model: &str,
    value: &Value,
) -> Result<AiResponse, AiError> {
    let model = value
        .get("model")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback_model)
        .to_owned();
    let text = match provider {
        AiProvider::Anthropic => decode_anthropic_text(value)?,
        AiProvider::OpenAi => decode_openai_text(value)?,
        AiProvider::Codex => decode_codex_text(value)?,
    };
    let (input_tokens, output_tokens) = decode_usage(provider, value);
    let command = first_shell_command(&text);
    Ok(AiResponse {
        provider,
        model,
        text,
        command,
        input_tokens,
        output_tokens,
    })
}

fn decode_anthropic_text(value: &Value) -> Result<String, AiError> {
    let Some(blocks) = value.get("content").and_then(Value::as_array) else {
        return Err(AiError::InvalidResponse(
            "Anthropic response missing content blocks".to_owned(),
        ));
    };
    let parts: Vec<&str> = blocks
        .iter()
        .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|b| b.get("text").and_then(Value::as_str))
        .collect();
    if parts.is_empty() {
        return Err(AiError::InvalidResponse(
            "Anthropic response contained no text".to_owned(),
        ));
    }
    Ok(parts.join("\n"))
}

fn decode_openai_text(value: &Value) -> Result<String, AiError> {
    let content = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|c| c.first())
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    content.map(str::to_owned).ok_or_else(|| {
        AiError::InvalidResponse("OpenAI response missing choices[0].message.content".to_owned())
    })
}

fn decode_codex_text(value: &Value) -> Result<String, AiError> {
    if let Some(direct) = value.get("output_text").and_then(Value::as_str) {
        if !direct.trim().is_empty() {
            return Ok(direct.to_owned());
        }
    }
    let Some(items) = value.get("output").and_then(Value::as_array) else {
        return Err(AiError::InvalidResponse(
            "Codex response missing output text".to_owned(),
        ));
    };
    let mut chunks = Vec::new();
    for item in items {
        let Some(parts) = item.get("content").and_then(Value::as_array) else {
            continue;
        };
        for part in parts {
            if let Some(text) = part.get("text").and_then(Value::as_str) {
                chunks.push(text);
            }
        }
    }
    if chunks.is_empty() {
        return Err(AiError::InvalidResponse(
            "Codex response output contained no text".to_owned(),
        ));
    }
    Ok(chunks.join("\n"))
}

fn decode_usage(provider: AiProvider, value: &Value) -> (Option<u64>, Option<u64>) {
    let Some(usage) = value.get("usage") else {
        return (None, None);
    };
    match provider {
        AiProvider::Anthropic | AiProvider::Codex => (
            usage.get("input_tokens").and_then(Value::as_u64),
            usage.get("output_tokens").and_then(Value::as_u64),
        ),
        AiProvider::OpenAi => (
            usage.get("prompt_tokens").and_then(Value::as_u64),
            usage.get("completion_tokens").and_then(Value::as_u64),
        ),
    }
}
