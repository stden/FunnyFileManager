//! Purpose: Unit tests for AI provider configuration, auth resolution, and model defaults.

mod config_edges;
mod http;
mod http_edges;
mod providers;

use std::collections::BTreeMap;

use serde_json::Value;

use super::client::AiTransport;
use super::config::{load_default_settings, load_settings, resolve, resolve_with, AuthHeader};
use crate::values::ai::{AiError, AiProvider, AiSettings, WorkspaceContext};

struct StubAiTransport {
    reply: Result<Value, AiError>,
}

impl StubAiTransport {
    fn new(reply: Result<Value, AiError>) -> Self {
        Self { reply }
    }
}

impl AiTransport for StubAiTransport {
    fn post(&mut self, _path: &str, _body: &Value) -> Result<Value, AiError> {
        self.reply.clone()
    }
}

fn env_map(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
    let map: BTreeMap<String, String> = pairs
        .iter()
        .map(|&(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    move |key| map.get(key).cloned()
}

fn sample_ctx() -> WorkspaceContext {
    WorkspaceContext {
        active_dir: "/srv/work".into(),
        opposite_dir: "/srv/data".into(),
        selected: Some("notes.txt".into()),
        marked: vec!["a.rs".into(), "b.rs".into()],
        os: "windows",
    }
}

#[test]
fn provider_metadata_and_yaml_settings_round_trip() {
    assert_eq!(AiProvider::ALL.len(), 3);
    assert_eq!(AiProvider::Anthropic.next(), AiProvider::OpenAi);
    assert_eq!(AiProvider::OpenAi.next(), AiProvider::Codex);
    assert_eq!(AiProvider::Codex.next(), AiProvider::Anthropic);
    assert_eq!(AiProvider::parse("claude"), Some(AiProvider::Anthropic));
    assert_eq!(AiProvider::parse("GPT"), Some(AiProvider::OpenAi));
    assert_eq!(AiProvider::parse("codex"), Some(AiProvider::Codex));
    assert_eq!(AiProvider::parse("unknown"), None);
    assert_eq!(AiProvider::Codex.id(), "codex");

    let temp = tempfile::TempDir::new().expect("temp");
    let yml = temp.path().join("ffm.yml");
    assert_eq!(load_settings(&yml).expect("missing"), AiSettings::default());
    std::fs::write(
        &yml,
        "ai:\n  provider: codex\n  tls_verify: false\n  codex:\n    model: codex-mini\n",
    )
    .expect("yml");
    let loaded = load_settings(&yml).expect("loaded");
    assert_eq!(loaded.initial_provider(), AiProvider::Codex);
    assert_eq!(
        loaded.for_provider(AiProvider::Codex).model.as_deref(),
        Some("codex-mini")
    );
    drop(load_default_settings());
}

#[test]
fn resolve_selects_auth_modes_models_and_redacts_secrets() {
    let settings = AiSettings::default();
    let anth = resolve_with(
        AiProvider::Anthropic,
        &settings,
        env_map(&[("ANTHROPIC_API_KEY", "fixture-a-tok")]),
    )
    .expect("anth");
    assert_eq!(anth.auth, AuthHeader::AnthropicKey);
    assert!(!format!("{anth:?}").contains("fixture-a-tok"));
    assert!(!format!("{:?}", anth.secret).contains("fixture-a-tok"));

    let anth_bearer = resolve_with(
        AiProvider::Anthropic,
        &settings,
        env_map(&[
            ("ANTHROPIC_AUTH_TOKEN", "fixture-b-tok"),
            (
                "ANTHROPIC_BASE_URL",
                "https://gateway.example.test/anthropic/",
            ),
            ("ANTHROPIC_DEFAULT_MODEL", "claude-model-default"),
        ]),
    )
    .expect("bearer");
    assert_eq!(anth_bearer.auth, AuthHeader::Bearer);
    assert_eq!(
        anth_bearer.base_url,
        "https://gateway.example.test/anthropic"
    );
    assert_eq!(anth_bearer.model, "claude-model-default");
}

#[test]
fn resolve_supports_openai_and_codex() {
    let settings = AiSettings::default();
    let oai = resolve_with(
        AiProvider::OpenAi,
        &settings,
        env_map(&[("OPENAI_API_KEY", "fixture-o-tok")]),
    )
    .expect("oai");
    assert_eq!(oai.model, "gpt-5");
    let codex = resolve_with(
        AiProvider::Codex,
        &settings,
        env_map(&[
            ("OPENAI_API_KEY", "fixture-c-tok"),
            ("AI_TLS_VERIFY", "false"),
        ]),
    )
    .expect("codex");
    assert_eq!(
        (codex.model.as_str(), codex.tls_verify),
        ("gpt-5.3-codex", false)
    );
}

#[test]
fn resolve_rejects_missing_or_invalid_env() {
    let settings = AiSettings::default();
    assert!(matches!(
        resolve_with(AiProvider::OpenAi, &settings, env_map(&[])),
        Err(AiError::MissingKey(_))
    ));
    assert!(resolve_with(
        AiProvider::OpenAi,
        &settings,
        env_map(&[
            ("OPENAI_API_KEY", "a\nb"),
            ("OPENAI_BASE_URL", "https://api.openai.com")
        ])
    )
    .is_err());
    assert!(resolve_with(
        AiProvider::OpenAi,
        &settings,
        env_map(&[
            ("OPENAI_API_KEY", "ok"),
            ("OPENAI_BASE_URL", "http://bad.example.test")
        ])
    )
    .is_err());
    drop(resolve(AiProvider::OpenAi, &settings));
}
