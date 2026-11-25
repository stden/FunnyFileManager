//! Purpose: Comments, docs, strings, lookalike names and test-only code that are not dependencies.

use super::Case;

pub(super) const LINE_COMMENT: Case = Case {
    name: "line_comment",
    file: "src/ui/status.rs",
    source: "// crate::ops::refresh(app) belongs to the caller.
fn draw() {}",
    expected: &[],
};

pub(super) const BLOCK_COMMENT: Case = Case {
    name: "block_comment",
    file: "src/ui/status.rs",
    source: "/* crate::ops::refresh(app) belongs to the caller. */
fn draw() {}",
    expected: &[],
};

pub(super) const DOC_COMMENTS: Case = Case {
    name: "doc_comments",
    file: "src/ui/status.rs",
    source: "//! Reads state; [`crate::ops`] mutates it.

/// Unlike [`crate::ops::refresh`], this only reads.
fn draw() {}",
    expected: &[],
};

pub(super) const BLOCK_DOC_COMMENT: Case = Case {
    name: "block_doc_comment",
    file: "src/ui/status.rs",
    source: "/** Never calls crate::ops::refresh. */
fn draw() {}",
    expected: &[],
};

pub(super) const DOC_ATTRIBUTE: Case = Case {
    name: "doc_attribute",
    file: "src/ui/status.rs",
    source: r#"#[doc = "Never calls std::process::Command."]
fn draw() {}"#,
    expected: &[],
};

pub(super) const STRING_LITERAL: Case = Case {
    name: "string_literal",
    file: "src/ui/status.rs",
    source: r#"const HINT: &str = "std::fs is off limits here";"#,
    expected: &[],
};

pub(super) const RAW_STRING_LITERAL: Case = Case {
    name: "raw_string_literal",
    file: "src/ui/status.rs",
    source: r##"const HINT: &str = r#"use std::fs;"#;"##,
    expected: &[],
};

pub(super) const LOOKALIKE_OPTIONS: Case = Case {
    name: "lookalike_options",
    file: "src/ui/status.rs",
    source: "use crate::options::Theme;",
    expected: &[],
};

pub(super) const LOOKALIKE_PREFIX: Case = Case {
    name: "lookalike_prefix",
    file: "src/ui/status.rs",
    source: "use crate::editorconfig::Style;",
    expected: &[],
};

pub(super) const TEST_ONLY_INLINE_MODULE: Case = Case {
    name: "test_only_inline_module",
    file: "src/ui/status.rs",
    source: "#[cfg(test)]
mod tests {
    use crate::ops::refresh;
}",
    expected: &[],
};
