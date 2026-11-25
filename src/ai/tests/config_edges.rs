//! Purpose: Edge-case tests for AI URL validation, YAML loading, and `execute()` wiring.

use std::io::Write;
use std::net::TcpListener;
use std::path::Path;
use std::thread;

use super::super::config::{load_default_settings, resolve_with};
use super::super::execute;
use super::env_map;
use crate::values::ai::{
    AiError, AiProvider, AiRequest, AiSettings, ProviderSettings, WorkspaceContext,
};

fn loopback_reset() -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let handle = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            drop(stream.flush());
        }
    });
    (format!("https://{addr}"), handle)
}

#[test]
fn resolve_rejects_malformed_and_credentialed_base_urls() {
    let settings = AiSettings::default();
    for bad_url in [
        "not a url",
        "https://user:pass@api.example.test",
        "https://api.example.test?q=1",
        "https://api.example.test#frag",
    ] {
        assert!(resolve_with(
            AiProvider::OpenAi,
            &settings,
            env_map(&[("OPENAI_API_KEY", "ok"), ("OPENAI_BASE_URL", bad_url)])
        )
        .is_err());
    }
}

#[test]
fn load_default_settings_handles_missing_home_and_reads_yaml() {
    {
        let _no_home = crate::test_support::EnvVarsGuard::set(&[
            ("XDG_CONFIG_HOME", Path::new("")),
            ("HOME", Path::new("")),
            ("APPDATA", Path::new("")),
        ]);
        assert_eq!(load_default_settings(), AiSettings::default());
    }

    let temp = tempfile::TempDir::new().expect("temp");
    let dir = temp.path().join("ffm");
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(
        dir.join(crate::config::document::FILE_NAME),
        "ai:\n  provider: codex\n",
    )
    .expect("yaml");
    let _home =
        crate::test_support::EnvVarGuard::set(crate::test_support::CONFIG_HOME, temp.path());
    assert_eq!(load_default_settings().provider.as_deref(), Some("codex"));
}

#[test]
fn execute_with_key_rejects_missing_ca_bundle() {
    let _env = crate::test_support::EnvVarsGuard::set(&[
        ("ANTHROPIC_API_KEY", Path::new("tok")),
        ("ANTHROPIC_BASE_URL", Path::new("")),
        ("AI_CA_CERT", Path::new("")),
    ]);
    let settings = AiSettings {
        ca_cert: Some("/nonexistent/ca.pem".into()),
        tls_verify: Some(true),
        ..AiSettings::default()
    };
    let req = AiRequest {
        provider: AiProvider::Anthropic,
        messages: Vec::new(),
        context: WorkspaceContext::default(),
        settings,
    };
    assert!(matches!(execute(&req), Err(AiError::Config(_))));
}

fn openai_loopback_request(base_url: String) -> AiRequest {
    AiRequest {
        provider: AiProvider::OpenAi,
        messages: Vec::new(),
        context: WorkspaceContext::default(),
        settings: AiSettings {
            openai: ProviderSettings {
                base_url: Some(base_url),
                ..ProviderSettings::default()
            },
            tls_verify: Some(false),
            ..AiSettings::default()
        },
    }
}

#[test]
fn execute_reaches_transport_builder_and_loopback_network_error() {
    let _env = crate::test_support::EnvVarsGuard::set(&[
        ("OPENAI_API_KEY", Path::new("tok")),
        ("OPENAI_BASE_URL", Path::new("")),
        ("AI_CA_CERT", Path::new("")),
        ("AI_TLS_VERIFY", Path::new("")),
        ("NO_PROXY", Path::new("*")),
        ("HTTPS_PROXY", Path::new("")),
        ("https_proxy", Path::new("")),
        ("ALL_PROXY", Path::new("")),
        ("all_proxy", Path::new("")),
    ]);
    let (https_loopback, handle) = loopback_reset();
    let req = openai_loopback_request(https_loopback);
    assert!(matches!(execute(&req), Err(AiError::Network(_))));
    drop(handle.join());
}
