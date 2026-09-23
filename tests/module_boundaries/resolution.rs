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
    Case {
        name: "grouped_std_import",
        file: "src/ui/status.rs",
        source: "use std::{io, fs};",
        expected: &["std::fs"],
    },
    Case {
        name: "nested_group_import",
        file: "src/ui/status.rs",
        source: "use std::{io::{self, Write}, fs::File};",
        expected: &["std::fs::File"],
    },
    Case {
        name: "grouped_crate_import",
        file: "src/ui/status.rs",
        source: "use crate::{models, ops::file};",
        expected: &["crate::ops::file"],
    },
    Case {
        name: "renamed_std_module",
        file: "src/ui/status.rs",
        source: r#"use std::fs as f;

fn load() -> Vec<u8> {
    f::read("x").unwrap_or_default()
}"#,
        expected: &["std::fs", "std::fs::read"],
    },
    Case {
        name: "renamed_crate_module",
        file: "src/ui/status.rs",
        source: "use crate::ops as o;

fn copy(app: &mut App) {
    let _ = o::handle_copy(app);
}",
        expected: &["crate::ops", "crate::ops::handle_copy"],
    },
    Case {
        name: "renamed_crate_root",
        file: "src/ui/status.rs",
        source: "use crate as c;

fn refresh(app: &mut App) {
    c::ops::refresh(app);
}",
        expected: &["crate::ops::refresh"],
    },
    Case {
        name: "grandparent_module",
        file: "src/ui/panel.rs",
        source: "use super::super::ops;",
        expected: &["crate::ops"],
    },
    Case {
        name: "great_grandparent_group",
        file: "src/popup/nested/commands.rs",
        source: "use super::super::super::{ops, handlers};",
        expected: &["crate::ops", "crate::handlers"],
    },
    Case {
        name: "inline_module_parent",
        file: "src/ui.rs",
        source: "mod widgets {
    pub fn refresh(app: &mut App) {
        super::super::ops::refresh(app);
    }
}",
        expected: &["crate::ops::refresh"],
    },
    Case {
        name: "self_child_lookalike",
        file: "src/ui.rs",
        source: r#"mod fs {
    pub fn label() -> &'static str {
        "fs"
    }
}

fn title() -> &'static str {
    self::fs::label()
}

fn heading() -> &'static str {
    fs::label()
}"#,
        expected: &[],
    },
    Case {
        name: "absolute_expression_path",
        file: "src/ui/status.rs",
        source: "fn read(path: &Path) -> String {
    ::std::fs::read_to_string(path).unwrap_or_default()
}",
        expected: &["std::fs::read_to_string"],
    },
    Case {
        name: "qualified_constructor",
        file: "src/action/keys.rs",
        source: r#"fn spawn() {
    let _ = std::process::Command::new("ls");
}"#,
        expected: &["std::process::Command::new"],
    },
    Case {
        name: "qualified_parameter_type",
        file: "src/ui/status.rs",
        source: "fn show(_: crate::ops::Outcome) {}",
        expected: &["crate::ops::Outcome"],
    },
    Case {
        name: "qualified_impl_types",
        file: "src/ui/status.rs",
        source: "impl From<crate::ops::Outcome> for Status {
    fn from(_: crate::ops::Outcome) -> Self {
        Self
    }
}",
        expected: &["crate::ops::Outcome", "crate::ops::Outcome"],
    },
    Case {
        name: "generic_argument",
        file: "src/ui/status.rs",
        source: "fn pending() -> Vec<crate::ops::Outcome> {
    Vec::new()
}",
        expected: &["crate::ops::Outcome"],
    },
    Case {
        name: "macro_argument",
        file: "src/ui/status.rs",
        source: r#"fn render(path: &Path) -> std::io::Result<String> {
    Ok(format!("{}", std::fs::read_to_string(path)?))
}"#,
        expected: &["std::fs::read_to_string"],
    },
    Case {
        name: "macro_rules_body",
        file: "src/action/keys.rs",
        source: "macro_rules! refresh {
    ($app:expr) => {
        $crate::ops::refresh($app)
    };
}",
        expected: &["crate::ops::refresh"],
    },
    Case {
        name: "block_scoped_import",
        file: "src/ui/status.rs",
        source: r#"fn load() -> Vec<u8> {
    use std::fs;
    fs::read("x").unwrap_or_default()
}"#,
        expected: &["std::fs", "std::fs::read"],
    },
    Case {
        name: "glob_import",
        file: "src/ui/status.rs",
        source: "use crate::ops::*;",
        expected: &["crate::ops::*"],
    },
    Case {
        name: "string_hides_code",
        file: "src/ui/status.rs",
        source: r#"fn wipe() {
    let _ = ("//", std::fs::remove_file("x"));
}"#,
        expected: &["std::fs::remove_file"],
    },
    Case {
        name: "line_comment",
        file: "src/ui/status.rs",
        source: "// crate::ops::refresh(app) belongs to the caller.
fn draw() {}",
        expected: &[],
    },
    Case {
        name: "block_comment",
        file: "src/ui/status.rs",
        source: "/* crate::ops::refresh(app) belongs to the caller. */
fn draw() {}",
        expected: &[],
    },
    Case {
        name: "doc_comments",
        file: "src/ui/status.rs",
        source: "//! Reads state; [`crate::ops`] mutates it.

/// Unlike [`crate::ops::refresh`], this only reads.
fn draw() {}",
        expected: &[],
    },
    Case {
        name: "block_doc_comment",
        file: "src/ui/status.rs",
        source: "/** Never calls crate::ops::refresh. */
fn draw() {}",
        expected: &[],
    },
    Case {
        name: "doc_attribute",
        file: "src/ui/status.rs",
        source: r#"#[doc = "Never calls std::process::Command."]
fn draw() {}"#,
        expected: &[],
    },
    Case {
        name: "string_literal",
        file: "src/ui/status.rs",
        source: r#"const HINT: &str = "std::fs is off limits here";"#,
        expected: &[],
    },
    Case {
        name: "raw_string_literal",
        file: "src/ui/status.rs",
        source: r##"const HINT: &str = r#"use std::fs;"#;"##,
        expected: &[],
    },
    Case {
        name: "lookalike_options",
        file: "src/ui/status.rs",
        source: "use crate::options::Theme;",
        expected: &[],
    },
    Case {
        name: "lookalike_prefix",
        file: "src/ui/status.rs",
        source: "use crate::editorconfig::Style;",
        expected: &[],
    },
    Case {
        name: "crate_root_reexport",
        file: "src/ui/status.rs",
        source: "fn restart() {
    crate::run_app();
}",
        expected: &["crate::application::run_app"],
    },
    Case {
        name: "test_only_inline_module",
        file: "src/ui/status.rs",
        source: "#[cfg(test)]
mod tests {
    use crate::ops::refresh;
}",
        expected: &[],
    },
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
