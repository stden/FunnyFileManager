//! Purpose: The crate root and the compatibility facade only wire modules.

use super::*;

/// The crate root and `handlers` only declare modules and re-exports.
pub fn facade_rule_violations(sources: &[SourceFile]) -> Vec<String> {
    sources
        .iter()
        .filter(|file| file.relative == "src/lib.rs" || file.in_module("handlers"))
        .flat_map(facade_violations)
        .collect()
}

/// `handlers` only forwards `crate::ops` and `crate::popup`.
pub fn handlers_forwarding_violations(sources: &[SourceFile]) -> Vec<String> {
    let mut messages = Vec::new();
    for file in sources.iter().filter(|file| file.in_module("handlers")) {
        let foreign = dependencies(file).into_iter().filter(|dependency| {
            !["crate::ops", "crate::popup"]
                .iter()
                .any(|target| starts_with_path(&dependency.path, target))
        });
        messages.extend(foreign.map(|dependency| {
            format!(
                "{}:{}: facade re-exports `{}`; it only forwards `crate::ops` and `crate::popup`",
                file.relative, dependency.line, dependency.path
            )
        }));
    }
    messages
}

#[test]
fn facade_rule_reports_behaviour_test_code_and_foreign_reexports() {
    let sources = sources(&[
        ("src/lib.rs", "pub mod handlers;\n\npub fn run() {}\n"),
        (
            "src/handlers/mod.rs",
            "pub use crate::ops::handle_copy;\npub use crate::models::App;\n\n#[cfg(test)]\nmod tests;\n\npub mod shell {\n    pub fn run() {}\n}\n",
        ),
    ]);
    assert_eq!(
        facade_rule_violations(&sources),
        [
            "src/lib.rs:3: facade owns an item other than `mod` or `use`",
            "src/handlers/mod.rs:4: facade carries `#[cfg(test)]` code",
            "src/handlers/mod.rs:8: facade owns an item other than `mod` or `use`",
        ]
    );
    assert_eq!(
        handlers_forwarding_violations(&sources),
        ["src/handlers/mod.rs:2: facade re-exports `crate::models::App`; it only forwards `crate::ops` and `crate::popup`"]
    );
}
