//! Purpose: Pin how dependency paths in Rust source resolve, one syntax form per fixture.
//!
//! Every fixture is judged by the rendering rule's forbidden dependencies
//! (`std::fs`, `std::process`, `crate::ops`, `crate::handlers`,
//! `crate::editor`, ...). `expected` lists the forbidden paths, fully resolved,
//! the checker must report, in source order.
//!
//! The substring matcher these guards used before (`contains` on the source
//! with `//` comments and whitespace stripped) misread fifteen fixtures:
//! it missed `grouped_std_import`, `nested_group_import`,
//! `grouped_crate_import`, `renamed_crate_root`, `grandparent_module`,
//! `great_grandparent_group`, `inline_module_parent` and `string_hides_code`
//! (and `crate_root_reexport`, added with the checker), and flagged
//! `block_comment`, `block_doc_comment`, `doc_attribute`, `string_literal`,
//! `raw_string_literal`, `lookalike_prefix` and `test_only_inline_module`.

use super::architecture::{forbidden_dependencies, SourceFile};

#[path = "resolution/expressions.rs"]
mod expressions;
#[path = "resolution/imports.rs"]
mod imports;
#[path = "resolution/not_code.rs"]
mod not_code;
#[path = "resolution/relative_paths.rs"]
mod relative_paths;

/// The crate root the fixtures compile under; it re-exports one operation.
const LIB: &str = "mod application;\npub use application::run_app;\n";

struct Case {
    name: &'static str,
    /// Where the snippet lives; fixes its module path for `super::`/`self::`.
    file: &'static str,
    source: &'static str,
    expected: &'static [&'static str],
}

const CASES: &[Case] = &[
    imports::GROUPED_STD_IMPORT,
    imports::NESTED_GROUP_IMPORT,
    imports::GROUPED_CRATE_IMPORT,
    imports::RENAMED_STD_MODULE,
    imports::RENAMED_CRATE_MODULE,
    imports::RENAMED_CRATE_ROOT,
    relative_paths::GRANDPARENT_MODULE,
    relative_paths::GREAT_GRANDPARENT_GROUP,
    relative_paths::INLINE_MODULE_PARENT,
    relative_paths::SELF_CHILD_LOOKALIKE,
    expressions::ABSOLUTE_EXPRESSION_PATH,
    expressions::QUALIFIED_CONSTRUCTOR,
    expressions::QUALIFIED_PARAMETER_TYPE,
    expressions::QUALIFIED_IMPL_TYPES,
    expressions::GENERIC_ARGUMENT,
    expressions::MACRO_ARGUMENT,
    expressions::MACRO_RULES_BODY,
    expressions::BLOCK_SCOPED_IMPORT,
    expressions::GLOB_IMPORT,
    expressions::STRING_HIDES_CODE,
    not_code::LINE_COMMENT,
    not_code::BLOCK_COMMENT,
    not_code::DOC_COMMENTS,
    not_code::BLOCK_DOC_COMMENT,
    not_code::DOC_ATTRIBUTE,
    not_code::STRING_LITERAL,
    not_code::RAW_STRING_LITERAL,
    not_code::LOOKALIKE_OPTIONS,
    not_code::LOOKALIKE_PREFIX,
    relative_paths::CRATE_ROOT_REEXPORT,
    not_code::TEST_ONLY_INLINE_MODULE,
];

fn reported(case: &Case) -> Vec<String> {
    let sources = [
        SourceFile::new("src/lib.rs", LIB),
        SourceFile::new(case.file, case.source),
    ];
    let checked = |file: &SourceFile| file.relative == case.file;
    forbidden_dependencies(&sources, checked, &super::layers::RENDERING_FORBIDDEN)
        .into_iter()
        .map(|violation| violation.path)
        .collect()
}

#[test]
fn checker_resolves_every_fixture() {
    for case in CASES {
        assert_eq!(reported(case), case.expected, "{}", case.name);
    }
}

#[test]
fn fixtures_are_uniquely_named_and_placed_in_checked_modules() {
    let mut names = CASES.iter().map(|case| case.name).collect::<Vec<_>>();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), CASES.len());
    assert!(CASES
        .iter()
        .all(|case| ["src/ui", "src/action", "src/popup"]
            .iter()
            .any(|module| case.file.starts_with(module))));
}
