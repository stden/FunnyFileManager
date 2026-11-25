//! Purpose: Unit tests for AI provider identifiers, defaults, and settings accessors.

use super::{AiProvider, AiSettings, ProviderSettings};

#[test]
fn provider_ids_labels_models_and_urls_cover_all_variants() {
    assert_eq!(AiProvider::Anthropic.id(), "anthropic");
    assert_eq!(AiProvider::OpenAi.id(), "openai");
    assert_eq!(AiProvider::Codex.id(), "codex");
    assert_eq!(AiProvider::Anthropic.label(), "Anthropic");
    assert_eq!(AiProvider::OpenAi.label(), "OpenAI");
    assert_eq!(AiProvider::Codex.label(), "Codex");
    assert_eq!(AiProvider::Anthropic.default_model(), "claude-sonnet-4-5");
    assert_eq!(AiProvider::OpenAi.default_model(), "gpt-5");
    assert_eq!(AiProvider::Codex.default_model(), "gpt-5.3-codex");
    assert_eq!(
        AiProvider::Anthropic.default_base_url(),
        "https://api.anthropic.com"
    );
    assert_eq!(
        AiProvider::OpenAi.default_base_url(),
        "https://api.openai.com"
    );
}

fn sample_settings() -> AiSettings {
    AiSettings {
        provider: Some("gpt".to_owned()),
        anthropic: ProviderSettings {
            model: Some("a".to_owned()),
            ..ProviderSettings::default()
        },
        openai: ProviderSettings {
            model: Some("o".to_owned()),
            ..ProviderSettings::default()
        },
        codex: ProviderSettings {
            model: Some("c".to_owned()),
            ..ProviderSettings::default()
        },
        ..AiSettings::default()
    }
}

#[test]
fn settings_lookup_selects_each_provider_and_initial_default() {
    let settings = sample_settings();
    assert_eq!(settings.initial_provider(), AiProvider::OpenAi);
    assert_eq!(
        settings
            .for_provider(AiProvider::Anthropic)
            .model
            .as_deref(),
        Some("a")
    );
    assert_eq!(
        settings.for_provider(AiProvider::OpenAi).model.as_deref(),
        Some("o")
    );
    assert_eq!(
        settings.for_provider(AiProvider::Codex).model.as_deref(),
        Some("c")
    );
    assert_eq!(
        AiSettings::default().initial_provider(),
        AiProvider::Anthropic
    );
}
