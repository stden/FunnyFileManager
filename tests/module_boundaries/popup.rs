//! Purpose: Guard every production popup module against compatibility dependencies.

use super::architecture::{forbidden_dependencies, SourceFile};

fn compatibility_dependencies(sources: &[SourceFile]) -> Vec<String> {
    let checked = |file: &SourceFile| !file.is_test() && file.in_module("popup");
    super::messages(&forbidden_dependencies(
        sources,
        checked,
        &["crate::handlers"],
    ))
}

#[test]
fn popup_does_not_depend_on_compatibility_handlers() {
    assert_eq!(
        compatibility_dependencies(&super::tui_sources()),
        Vec::<String>::new()
    );
}

#[test]
fn compatibility_dependencies_in_nested_popup_modules_are_rejected() {
    let sources = [SourceFile::new(
        "src/popup/nested/commands.rs",
        "use crate::handlers::handle_copy;\n",
    )];
    assert_eq!(
        compatibility_dependencies(&sources),
        ["src/popup/nested/commands.rs:1: `crate::handlers::handle_copy` (forbidden: `crate::handlers`)"]
    );
}

#[test]
fn compatibility_dependencies_in_popup_root_are_rejected() {
    let sources = [SourceFile::new("src/popup.rs", "use super::{handlers};\n")];
    assert_eq!(
        compatibility_dependencies(&sources),
        ["src/popup.rs:1: `crate::handlers` (forbidden: `crate::handlers`)"]
    );
}

#[test]
fn relative_paths_resolve_from_the_popup_module() {
    // From `popup::nested::commands`, two `super`s reach `popup` itself.
    let sources = [SourceFile::new(
        "src/popup/nested/commands.rs",
        "use super::super::handlers::Local;\nuse super::super::super::{ops, handlers};\n",
    )];
    assert_eq!(
        compatibility_dependencies(&sources),
        ["src/popup/nested/commands.rs:2: `crate::handlers` (forbidden: `crate::handlers`)"]
    );
}

#[test]
fn popup_test_sources_and_comments_do_not_trigger_dependency_guard() {
    let sources = [
        SourceFile::new("src/popup/nested/tests.rs", "use crate::handlers;\n"),
        SourceFile::new(
            "src/popup/nested/commands.rs",
            "// crate::handlers is forbidden\n/* crate::handlers */\nconst NOTE: &str = \"crate::handlers\";\nuse crate::ops::handle_copy;\n",
        ),
    ];
    assert_eq!(compatibility_dependencies(&sources).len(), 0);
}
