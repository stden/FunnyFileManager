//! Purpose: Test attributes live in test files, and test helpers never pick the runtime.

use super::*;

/// Test bodies live in `cfg(test)` child files, never in production sources.
pub fn embedded_test_violations(sources: &[SourceFile]) -> Vec<String> {
    sources
        .iter()
        .filter(|file| !file.is_test())
        .flat_map(|file| {
            test_attribute_lines(file).into_iter().map(|line| {
                format!(
                    "{}:{line}: test attribute in production code; use a cfg(test) child file",
                    file.relative
                )
            })
        })
        .collect()
}

/// `name` is defined once in `file`, and never chosen by `feature = "test-helpers"`.
pub fn feature_selection_violations(file: &SourceFile, name: &str) -> Vec<String> {
    let definitions = functions(file)
        .into_iter()
        .filter(|function| function.name == name)
        .collect::<Vec<_>>();
    if definitions.len() != 1 {
        let count = definitions.len();
        return vec![format!(
            "{}: `{name}` has {count} definitions; expected one",
            file.relative
        )];
    }
    definitions
        .iter()
        .filter(|function| function.selects_test_helpers)
        .map(|function| {
            let line = function.lines.start();
            format!(
                "{}:{line}: `{name}` is selected by the `test-helpers` feature",
                file.relative
            )
        })
        .collect()
}

#[test]
fn embedded_test_rule_reports_test_attributes_outside_test_files() {
    let sources = [
        SourceFile::new(
            "src/ops/file.rs",
            "pub fn copy() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn copies() {}\n\n    #[tokio::test]\n    async fn copies_later() {}\n}\n",
        ),
        SourceFile::new("src/ops/file/tests.rs", "#[test]\nfn copies() {}\n"),
        SourceFile::new("src/ops/hint.rs", "pub const HINT: &str = \"#[test]\";\n"),
    ];
    assert_eq!(
        embedded_test_violations(&sources),
        [
            "src/ops/file.rs:5: test attribute in production code; use a cfg(test) child file",
            "src/ops/file.rs:8: test attribute in production code; use a cfg(test) child file",
        ]
    );
}

#[test]
fn feature_selection_rule_reports_duplicates_and_feature_checks() {
    let duplicated = SourceFile::new(
        "src/bootstrap.rs",
        "#[cfg(feature = \"test-helpers\")]\npub fn run() {}\n\n#[cfg(not(feature = \"test-helpers\"))]\npub fn run() {}\n",
    );
    assert_eq!(
        feature_selection_violations(&duplicated, "run"),
        ["src/bootstrap.rs: `run` has 2 definitions; expected one"]
    );
    let branching = SourceFile::new(
        "src/editor.rs",
        "pub fn handle_edit<B>(terminal: &mut B) {\n    if cfg!(feature = \"test-helpers\") {\n        return;\n    }\n}\n",
    );
    assert_eq!(
        feature_selection_violations(&branching, "handle_edit"),
        ["src/editor.rs:1: `handle_edit` is selected by the `test-helpers` feature"]
    );
    let documented = SourceFile::new(
        "src/bootstrap.rs",
        "/// Unlike the `feature = \"test-helpers\"` seams, this is the real entry.\npub fn run() {}\n",
    );
    assert_eq!(feature_selection_violations(&documented, "run").len(), 0);
}
