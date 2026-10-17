//! Purpose: distinguish shared tooling references from private paths, contacts, and credentials.

use super::Rules;
use regex::Regex;

fn findings(text: &str) -> Vec<&'static str> {
    let rules = Rules::with_private_words(Vec::new()).unwrap();
    let mut findings = Vec::new();
    rules.check(text, |rule, _| findings.push(rule));
    findings
}

#[test]
fn credential_prefixes_inside_words_are_not_secrets() {
    assert!(findings("risk-prioritization-rationale.md").is_empty());
}

#[test]
fn standalone_credentials_are_still_rejected() {
    let samples = [
        ("sk-", "a".repeat(24)),
        ("ghp_", "a".repeat(36)),
        ("github_pat_", "a".repeat(24)),
        ("AKIA", "A".repeat(16)),
        ("xoxb-", "a".repeat(16)),
        ("AIza", "a".repeat(35)),
    ];
    for (prefix, suffix) in samples {
        let text = format!("api_key = \"{prefix}{suffix}\"");
        assert_eq!(findings(&text), ["credential"]);
    }
}

#[test]
fn github_shared_ssh_login_is_not_a_personal_email() {
    assert!(findings("git@github.com:example/project.git").is_empty());
}

#[test]
fn personal_contacts_on_the_same_host_are_still_rejected() {
    let contact = format!("{}@{}", "private-person", "github.com");
    assert_eq!(findings(&contact), ["personal e-mail"]);
}

#[test]
fn linuxbrew_standard_installation_is_a_shared_path() {
    assert!(findings("/home/linuxbrew/.linuxbrew/bin").is_empty());
}

#[test]
fn other_home_paths_are_still_rejected_alongside_shared_paths() {
    let paths = [
        format!("{}/{}/projects", "/home", "linuxbrew"),
        format!("{}/{}", "/home", "private-person"),
    ];
    for path in paths {
        let text = format!("/home/linuxbrew/.linuxbrew/bin {path}");
        assert_eq!(findings(&text), ["machine-specific path"]);
    }
}

#[test]
fn injected_private_words_remain_rejected() {
    let pattern = Regex::new(r"(?i)\bconfidential\b").unwrap();
    let rules = Rules::with_private_words(vec![pattern]).unwrap();
    let mut findings = Vec::new();
    rules.check("CONFIDENTIAL", |rule, _| findings.push(rule));
    assert_eq!(findings, ["private word"]);
}

#[test]
fn bare_windows_workspace_paths_are_rejected() {
    let drive_path = format!(r"{}\{}", "C:", "x");
    let escaped_path = format!(r"{}\\{}", "C:", "x");
    for sample in [drive_path, escaped_path] {
        assert_eq!(findings(&sample), ["machine-specific path"], "{sample}");
    }
}
