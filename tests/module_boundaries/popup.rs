//! Purpose: Guard every production popup module against compatibility dependencies.

use std::fs;
use std::path::{Path, PathBuf};

fn compatibility_dependencies(source_root: &Path) -> Vec<PathBuf> {
    let mut files = vec![source_root.join("popup.rs")];
    super::rust_files(&source_root.join("popup"), &mut files);
    files.sort();
    files
        .into_iter()
        .filter(|path| !super::is_test_source(path))
        .filter(|path| {
            let code = super::compact_code(path);
            [
                "crate::handlers",
                "crate::{handlers",
                "super::handlers",
                "super::{handlers",
            ]
            .iter()
            .any(|dependency| code.contains(dependency))
        })
        .collect()
}

#[test]
fn popup_does_not_depend_on_compatibility_handlers() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let violations = compatibility_dependencies(&source_root);
    assert!(
        violations.is_empty(),
        "popup modules depend on compatibility handlers: {violations:?}"
    );
}

fn fixture() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("temporary source tree");
    fs::write(directory.path().join("popup.rs"), "mod nested;\n").expect("popup root");
    fs::create_dir_all(directory.path().join("popup/nested")).expect("nested popup modules");
    directory
}

#[test]
fn compatibility_dependencies_in_nested_popup_modules_are_rejected() {
    let directory = fixture();
    let nested = directory.path().join("popup/nested/commands.rs");
    fs::write(&nested, "use crate::handlers::handle_copy;\n").expect("nested dependency");
    assert_eq!(compatibility_dependencies(directory.path()), vec![nested]);
}

#[test]
fn compatibility_dependencies_in_popup_root_are_rejected() {
    let directory = fixture();
    let root = directory.path().join("popup.rs");
    fs::write(&root, "use super::{handlers};\n").expect("root dependency");
    assert_eq!(compatibility_dependencies(directory.path()), vec![root]);
}

#[test]
fn popup_test_sources_and_comments_do_not_trigger_dependency_guard() {
    let directory = fixture();
    let nested = directory.path().join("popup/nested");
    fs::write(nested.join("tests.rs"), "use crate::handlers;\n").expect("test dependency");
    fs::write(
        nested.join("commands.rs"),
        "// crate::handlers is forbidden\nuse crate::ops::handle_copy;\n",
    )
    .expect("canonical production dependency");
    assert!(compatibility_dependencies(directory.path()).is_empty());
}
