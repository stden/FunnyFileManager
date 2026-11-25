//! Purpose: Jira config resolution, token redaction, and URL validation unit tests.

use super::*;
use crate::jira::config::{
    load_default_settings, load_settings, resolve_config, resolve_config_with, validate_url,
};

#[test]
fn config_resolution_validates_urls_tokens_tls_and_yaml() {
    let temp = tempfile::TempDir::new().expect("temp");
    let yml = temp.path().join("ffm.yml");
    assert_eq!(
        load_settings(&yml).expect("missing"),
        JiraSettings::default()
    );
    std::fs::write(
        &yml,
        "jira:\n  url: https://jira.example.test/\n  tls_verify: false\n  page_size: 25\n",
    )
    .expect("yaml");
    let loaded = load_settings(&yml).expect("loaded");
    assert_eq!(loaded.tls_verify, Some(false));
    let cfg =
        resolve_config_with(&loaded, env_lookup(&[("JIRA_PAT", "tok-123")])).expect("resolved");
    assert_eq!(
        (cfg.base_url.as_str(), cfg.tls_verify, cfg.page_size),
        ("https://jira.example.test", false, 25)
    );
    assert!(!format!("{cfg:?}").contains("tok-123"));
    assert!(!format!("{:?}", cfg.token).contains("tok-123"));
}

#[test]
fn config_rejects_bad_urls_control_chars_and_missing_env() {
    let s = JiraSettings::default();
    assert!(resolve_config_with(&s, env_lookup(&[])).is_err());
    let no_tok = resolve_config_with(&s, env_lookup(&[("JIRA_URL", "https://jira.example.test")]));
    assert!(no_tok
        .unwrap_err()
        .to_string()
        .contains("JIRA_PERSONAL_TOKEN"));
    for bad in [
        "http://jira.example.test",
        "not a url",
        "https://user:pass@jira.example.test",
        "https://jira.example.test?q=1",
    ] {
        assert!(validate_url(bad).is_err());
    }
    let bad_tok = resolve_config_with(
        &s,
        env_lookup(&[
            ("JIRA_URL", "https://jira.example.test"),
            ("JIRA_TOKEN", "a\nb"),
        ]),
    );
    assert!(bad_tok.is_err());
}

#[test]
fn config_allows_tls_env_override() {
    let tls_on = JiraSettings {
        url: Some("https://jira.example.test".into()),
        tls_verify: Some(false),
        ..JiraSettings::default()
    };
    let overr = resolve_config_with(
        &tls_on,
        env_lookup(&[("JIRA_TOKEN", "ok"), ("JIRA_TLS_VERIFY", "true")]),
    )
    .expect("tls override");
    assert!(overr.tls_verify);
    drop(load_default_settings());
    drop(resolve_config(&JiraSettings::default()));
}

#[test]
fn execute_rejects_missing_ca_bundle() {
    let cfg = RuntimeConfig {
        base_url: "https://jira.example.test".into(),
        token: super::super::config::Token::try_new_for_test("secret"),
        tls_verify: true,
        ca_cert: Some("/nonexistent/ca.pem".into()),
        page_size: 10,
    };
    let reply = execute(
        &cfg,
        JiraRequest::Fetch {
            jql: "x".into(),
            page_size: 10,
        },
    );
    assert!(matches!(
        reply,
        JiraReply::Fetched(Err(JiraError::Config(_)))
    ));
}

#[test]
fn load_default_settings_handles_missing_home_and_reads_yaml() {
    use std::path::Path;
    {
        let _no_home = crate::test_support::EnvVarsGuard::set(&[
            ("XDG_CONFIG_HOME", Path::new("")),
            ("HOME", Path::new("")),
            ("APPDATA", Path::new("")),
        ]);
        assert_eq!(load_default_settings(), JiraSettings::default());
    }
    let temp = tempfile::TempDir::new().expect("temp");
    let dir = temp.path().join("ffm");
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(
        dir.join(crate::config::document::FILE_NAME),
        "jira:\n  url: https://jira.example.test\n",
    )
    .expect("yaml");
    let _home =
        crate::test_support::EnvVarGuard::set(crate::test_support::CONFIG_HOME, temp.path());
    assert_eq!(
        load_default_settings().url.as_deref(),
        Some("https://jira.example.test")
    );
}
