//! Purpose: The Cloudflare DNS code keeps its layers. The client never reaches the screen, the
//! operations or the input; the editor's state and its screen never reach the network or the
//! client; and no Cloudflare source names a DELETE request.

use super::architecture::{forbidden_dependencies, SourceFile, Violation};
use super::{messages, tui_sources};

/// The client, its codec and its validation are below the screen and the operations.
const CLIENT_FORBIDDEN: [&str; 6] = [
    "crate::ui",
    "crate::popup",
    "crate::ops",
    "crate::application",
    "crate::action",
    "crate::keymap",
];

/// Spellings that would name a DELETE request, in any form the code could take.
const DELETE_SPELLINGS: [&str; 4] = ["\"DELETE\"", ".delete(", "Method::Delete", "Delete =>"];

fn is_cloudflare(file: &SourceFile) -> bool {
    file.relative.contains("cloudflare")
}

pub fn client_violations(sources: &[SourceFile]) -> Vec<String> {
    let checked = |file: &SourceFile| !file.is_test() && file.in_module("cloudflare_dns");
    messages(&forbidden_dependencies(sources, checked, &CLIENT_FORBIDDEN))
}

/// The editor's state and screen never reach the network or the client: the client is the only
/// place that talks to Cloudflare, and it is handed to the job, never called from a screen.
pub fn editor_violations(sources: &[SourceFile]) -> Vec<String> {
    let checked = |file: &SourceFile| {
        !file.is_test() && (file.in_module("models") || file.in_module("ui")) && is_cloudflare(file)
    };
    messages(&forbidden_dependencies(
        sources,
        checked,
        &["crate::cloudflare_dns", "reqwest"],
    ))
}

pub fn delete_violations(sources: &[SourceFile]) -> Vec<String> {
    sources
        .iter()
        .filter(|file| is_cloudflare(file))
        .flat_map(|file| {
            file.text
                .lines()
                .enumerate()
                .filter(|(_, line)| {
                    DELETE_SPELLINGS
                        .iter()
                        .any(|spelling| line.contains(spelling))
                })
                .map(move |(index, _)| {
                    format!("{}:{}: names a DELETE request", file.relative, index + 1)
                })
        })
        .collect()
}

#[test]
fn the_cloudflare_client_stays_below_the_screen_and_the_operations() {
    assert_eq!(client_violations(&tui_sources()), Vec::<String>::new());
}

#[test]
fn the_cloudflare_editor_never_reaches_the_network_or_the_client() {
    assert_eq!(editor_violations(&tui_sources()), Vec::<String>::new());
}

#[test]
fn no_cloudflare_source_names_a_delete_request() {
    assert_eq!(delete_violations(&tui_sources()), Vec::<String>::new());
}

#[test]
fn the_client_rule_reports_a_screen_dependency() {
    let sources = [SourceFile::new(
        "src/cloudflare_dns/client.rs",
        "use crate::ui::dialog;\n",
    )];
    let violations: Vec<Violation> = forbidden_dependencies(
        &sources,
        |file| file.in_module("cloudflare_dns"),
        &CLIENT_FORBIDDEN,
    );
    assert_eq!(violations.len(), 1);
}

#[test]
fn the_delete_rule_reports_a_delete_spelling_in_any_cloudflare_source() {
    let sources = [
        SourceFile::new(
            "src/cloudflare_dns/call.rs",
            "fn verb() -> &'static str {\n    \"DELETE\"\n}\n",
        ),
        SourceFile::new(
            "src/ops/cloudflare_dns.rs",
            "const NOTE: &str = \"read only\";\n",
        ),
    ];
    assert_eq!(
        delete_violations(&sources),
        ["src/cloudflare_dns/call.rs:2: names a DELETE request"]
    );
}
