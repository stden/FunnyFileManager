//! Purpose: Unit tests for AI provider request builders, response decoders, and command extraction.

use std::collections::BTreeMap;

use serde_json::json;

use super::super::config::{ApiSecret, AuthHeader, ResolvedProvider};
use super::super::extract::first_shell_command;
use super::super::providers::{build_request, decode_response, system_instructions};
use super::super::{execute_with, unavailable};
use super::{sample_ctx, StubAiTransport};
use crate::values::ai::{AiError, AiProvider, ChatMessage, Role};

fn provider_fixtures() -> [(AiProvider, &'static str, serde_json::Value); 3] {
    [
        (
            AiProvider::Anthropic,
            "/v1/messages",
            json!({"model": "claude-sonnet-4-5", "content": [{"type": "text", "text": "Run:\n```bash\n# comment\n$ ls -la\n```"}], "usage": {"input_tokens": 10, "output_tokens": 5}}),
        ),
        (
            AiProvider::OpenAi,
            "/v1/chat/completions",
            json!({"model": "gpt-5", "choices": [{"message": {"content": "```pwsh\nGet-ChildItem\n```"}}], "usage": {"prompt_tokens": 9, "completion_tokens": 4}}),
        ),
        (
            AiProvider::Codex,
            "/v1/responses",
            json!({"model": "gpt-5.3-codex", "output_text": "```sh\nfind . -name '*.rs'\n```", "usage": {"input_tokens": 8, "output_tokens": 3}}),
        ),
    ]
}

#[test]
fn request_builders_and_command_extractor_cover_all_three_apis() {
    let ctx = sample_ctx();
    assert!(system_instructions(&ctx, Some("Be brief")).contains("Be brief"));
    assert_eq!(
        (Role::User.prefix(), Role::Assistant.prefix()),
        ("You", "AI")
    );
    let msgs = vec![ChatMessage {
        role: Role::User,
        text: "list rs files".into(),
    }];
    for (provider, expected_path, reply) in provider_fixtures() {
        let resolved = sample_resolved(provider);
        assert_eq!(build_request(&resolved, &msgs, &ctx).0, expected_path);
        let mut tr = StubAiTransport::new(Ok(reply));
        let out = execute_with(&mut tr, &resolved, &msgs, &ctx).expect("exec");
        assert!(out.command.is_some() && out.input_tokens.is_some());
    }
}

#[test]
fn response_decoders_handle_codex_output_arrays_and_errors() {
    let nested = decode_response(
        AiProvider::Codex,
        "codex",
        &json!({"output": [{"content": [{"text": "hello"}]}]}),
    )
    .expect("nested");
    assert_eq!(nested.text, "hello");
    assert!(decode_response(AiProvider::Anthropic, "m", &json!({})).is_err());
    assert!(decode_response(AiProvider::OpenAi, "m", &json!({})).is_err());
    assert!(decode_response(AiProvider::Codex, "m", &json!({})).is_err());
    assert_eq!(
        (Role::User.as_api_str(), Role::Assistant.as_api_str()),
        ("user", "assistant")
    );
    assert!(sample_ctx().prompt_summary().contains("/srv/work"));
    for p in AiProvider::ALL {
        assert!(p.default_base_url().starts_with("https://"));
    }
    assert_eq!(first_shell_command("```json\n{}\n```"), None);
    assert_eq!(unavailable().unwrap_err(), AiError::Disabled);
}

#[test]
fn response_decoders_reject_empty_anthropic_and_codex_payloads() {
    assert!(decode_response(
        AiProvider::Anthropic,
        "m",
        &json!({"content": [{"type": "thinking", "thinking": "..."}]}),
    )
    .is_err());
    assert!(decode_response(
        AiProvider::Codex,
        "m",
        &json!({
            "output_text": "   ",
            "output": [{"type": "reasoning"}, {"content": [{"type": "refusal"}]}]
        }),
    )
    .is_err());
    let mixed = decode_response(
        AiProvider::Codex,
        "m",
        &json!({"output": [{"type": "reasoning"}, {"content": [{"type": "refusal"}, {"text": "ok"}]}]}),
    )
    .expect("mixed");
    assert_eq!(mixed.text, "ok");
}

fn sample_resolved(provider: AiProvider) -> ResolvedProvider {
    ResolvedProvider {
        provider,
        base_url: "https://api.example.test".into(),
        model: provider.default_model().into(),
        max_tokens: 1024,
        auth: AuthHeader::Bearer,
        secret: ApiSecret::try_new_for_test("tok"),
        headers: BTreeMap::new(),
        tls_verify: true,
        ca_cert: None,
        system_prompt: None,
    }
}
