//! Purpose: Environment and YAML resolution with validation for Anthropic, `OpenAI`, and Codex.

use std::path::PathBuf;

use reqwest::Url;

use super::{ApiSecret, AuthHeader, ResolvedProvider};
use crate::values::ai::{AiError, AiProvider, AiSettings, ProviderSettings};
use crate::values::{non_blank, parse_tls_verify};

const DEFAULT_MAX_TOKENS: u32 = 4096;

/// Resolve provider settings using a caller-supplied env lookup.
pub fn resolve_with(
    provider: AiProvider,
    settings: &AiSettings,
    env: impl Fn(&str) -> Option<String>,
) -> Result<ResolvedProvider, AiError> {
    let cfg = settings.for_provider(provider);
    let base_url = validate_base_url(&resolve_base_url(provider, cfg, &env))?;
    let model = resolve_model(provider, cfg, &env);
    let (auth, secret) = resolve_secret(provider, &env)?;
    let max_tokens = cfg
        .max_tokens
        .unwrap_or(DEFAULT_MAX_TOKENS)
        .clamp(64, 32768);
    let tls_verify = parse_tls_verify(env("AI_TLS_VERIFY"), settings.tls_verify);
    let ca_cert = non_blank(env("AI_CA_CERT"))
        .map(PathBuf::from)
        .or_else(|| settings.ca_cert.clone());
    Ok(ResolvedProvider {
        provider,
        base_url,
        model,
        max_tokens,
        auth,
        secret,
        headers: cfg.headers.clone(),
        tls_verify,
        ca_cert,
        system_prompt: settings.system_prompt.clone(),
    })
}

fn resolve_base_url(
    provider: AiProvider,
    cfg: &ProviderSettings,
    env: &impl Fn(&str) -> Option<String>,
) -> String {
    let env_url = match provider {
        AiProvider::Anthropic => non_blank(env("ANTHROPIC_BASE_URL")),
        AiProvider::OpenAi => non_blank(env("OPENAI_BASE_URL")),
        AiProvider::Codex => {
            non_blank(env("CODEX_BASE_URL")).or_else(|| non_blank(env("OPENAI_BASE_URL")))
        }
    };
    env_url
        .or_else(|| non_blank(cfg.base_url.clone()))
        .unwrap_or_else(|| provider.default_base_url().to_owned())
}

fn resolve_model(
    provider: AiProvider,
    cfg: &ProviderSettings,
    env: &impl Fn(&str) -> Option<String>,
) -> String {
    let env_model = match provider {
        AiProvider::Anthropic => {
            non_blank(env("ANTHROPIC_MODEL")).or_else(|| non_blank(env("ANTHROPIC_DEFAULT_MODEL")))
        }
        AiProvider::OpenAi => non_blank(env("OPENAI_MODEL")),
        AiProvider::Codex => non_blank(env("CODEX_MODEL")),
    };
    env_model
        .or_else(|| non_blank(cfg.model.clone()))
        .unwrap_or_else(|| provider.default_model().to_owned())
}

fn resolve_secret(
    provider: AiProvider,
    env: &impl Fn(&str) -> Option<String>,
) -> Result<(AuthHeader, ApiSecret), AiError> {
    let candidates: &[(&str, AuthHeader)] = match provider {
        AiProvider::Anthropic => &[
            ("ANTHROPIC_API_KEY", AuthHeader::AnthropicKey),
            ("ANTHROPIC_AUTH_TOKEN", AuthHeader::Bearer),
        ],
        AiProvider::OpenAi => &[("OPENAI_API_KEY", AuthHeader::Bearer)],
        AiProvider::Codex => &[
            ("CODEX_API_KEY", AuthHeader::Bearer),
            ("OPENAI_API_KEY", AuthHeader::Bearer),
        ],
    };
    for &(var, auth) in candidates {
        if let Some(value) = non_blank(env(var)) {
            if value.chars().any(char::is_control) {
                return Err(AiError::Config(format!(
                    "{var} must not contain control characters"
                )));
            }
            return Ok((auth, ApiSecret::new(value)));
        }
    }
    let names = candidates
        .iter()
        .map(|&(v, _)| v)
        .collect::<Vec<_>>()
        .join(" or ");
    Err(AiError::MissingKey(format!(
        "Set {names} to use {} API",
        provider.label()
    )))
}

fn validate_base_url(raw: &str) -> Result<String, AiError> {
    let trimmed = raw.trim();
    let Ok(parsed) = Url::parse(trimmed) else {
        return Err(AiError::Config("AI base URL is invalid".to_owned()));
    };
    if parsed.scheme() != "https" {
        return Err(AiError::Config(
            "AI base URL must start with https://".to_owned(),
        ));
    }
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(AiError::Config(
            "AI base URL must not contain credentials, query, or fragment".to_owned(),
        ));
    }
    Ok(trimmed.trim_end_matches('/').to_owned())
}
