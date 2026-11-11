//! Purpose: Guard popup dependencies and keep name-prompt filesystem effects in ops.

use super::architecture::{forbidden_dependencies, SourceFile};

fn compatibility_dependencies(sources: &[SourceFile]) -> Vec<String> {
    let checked = |file: &SourceFile| !file.is_test() && file.in_module("popup");
    super::messages(&forbidden_dependencies(
        sources,
        checked,
        &["crate::handlers"],
    ))
}

fn name_prompt_filesystem_dependencies(sources: &[SourceFile]) -> Vec<String> {
    let checked = |file: &SourceFile| {
        !file.is_test()
            && file.in_module("popup")
            && file.module.get(1).is_some_and(|name| name == "name_prompt")
    };
    super::messages(&forbidden_dependencies(sources, checked, &["std::fs"]))
}

#[test]
fn name_prompt_delegates_filesystem_effects_to_ops() {
    assert_eq!(
        name_prompt_filesystem_dependencies(&super::tui_sources()),
        Vec::<String>::new()
    );
}

#[test]
fn name_prompt_filesystem_rename_and_directory_creation_are_rejected() {
    let sources = [SourceFile::new(
        "src/popup/name_prompt.rs",
        "fn apply() {\nstd::fs::rename(\"a\", \"b\");\nstd::fs::create_dir(\"b\");\nstd::fs::create_dir_all(\"b/c\");\n}\n",
    )];
    assert_eq!(
        name_prompt_filesystem_dependencies(&sources),
        [
            "src/popup/name_prompt.rs:2: `std::fs::rename` (forbidden: `std::fs`)",
            "src/popup/name_prompt.rs:3: `std::fs::create_dir` (forbidden: `std::fs`)",
            "src/popup/name_prompt.rs:4: `std::fs::create_dir_all` (forbidden: `std::fs`)",
        ]
    );
}

#[test]
fn name_prompt_nested_filesystem_alias_is_rejected() {
    let sources = [SourceFile::new(
        "src/popup/name_prompt/nested.rs",
        "use std::fs as files;\nfn apply() { files::create_dir(\"new\"); }\n",
    )];
    assert_eq!(
        name_prompt_filesystem_dependencies(&sources),
        [
            "src/popup/name_prompt/nested.rs:1: `std::fs` (forbidden: `std::fs`)",
            "src/popup/name_prompt/nested.rs:2: `std::fs::create_dir` (forbidden: `std::fs`)",
        ]
    );
}

#[test]
fn name_prompt_test_sources_and_comments_allow_filesystem_references() {
    let sources = [
        SourceFile::new("src/popup/name_prompt/tests.rs", "use std::fs;\n"),
        SourceFile::new(
            "src/popup/name_prompt.rs",
            "// std::fs is forbidden\nconst NOTE: &str = \"std::fs\";\n#[cfg(test)] fn setup() { std::fs::create_dir(\"test\"); }\n",
        ),
        SourceFile::new("src/ops/name_prompt.rs", "use std::fs;\n"),
    ];
    assert_eq!(
        name_prompt_filesystem_dependencies(&sources),
        Vec::<String>::new()
    );
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
