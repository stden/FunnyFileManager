//! Purpose: Paths inside expressions, types, generics, macros, blocks and glob imports.

use super::Case;

pub(super) const ABSOLUTE_EXPRESSION_PATH: Case = Case {
    name: "absolute_expression_path",
    file: "src/ui/status.rs",
    source: "fn read(path: &Path) -> String {
    ::std::fs::read_to_string(path).unwrap_or_default()
}",
    expected: &["std::fs::read_to_string"],
};

pub(super) const QUALIFIED_CONSTRUCTOR: Case = Case {
    name: "qualified_constructor",
    file: "src/action/keys.rs",
    source: r#"fn spawn() {
    let _ = std::process::Command::new("ls");
}"#,
    expected: &["std::process::Command::new"],
};

pub(super) const QUALIFIED_PARAMETER_TYPE: Case = Case {
    name: "qualified_parameter_type",
    file: "src/ui/status.rs",
    source: "fn show(_: crate::ops::Outcome) {}",
    expected: &["crate::ops::Outcome"],
};

pub(super) const QUALIFIED_IMPL_TYPES: Case = Case {
    name: "qualified_impl_types",
    file: "src/ui/status.rs",
    source: "impl From<crate::ops::Outcome> for Status {
    fn from(_: crate::ops::Outcome) -> Self {
        Self
    }
}",
    expected: &["crate::ops::Outcome", "crate::ops::Outcome"],
};

pub(super) const GENERIC_ARGUMENT: Case = Case {
    name: "generic_argument",
    file: "src/ui/status.rs",
    source: "fn pending() -> Vec<crate::ops::Outcome> {
    Vec::new()
}",
    expected: &["crate::ops::Outcome"],
};

pub(super) const MACRO_ARGUMENT: Case = Case {
    name: "macro_argument",
    file: "src/ui/status.rs",
    source: r#"fn render(path: &Path) -> std::io::Result<String> {
    Ok(format!("{}", std::fs::read_to_string(path)?))
}"#,
    expected: &["std::fs::read_to_string"],
};

pub(super) const MACRO_RULES_BODY: Case = Case {
    name: "macro_rules_body",
    file: "src/action/keys.rs",
    source: "macro_rules! refresh {
    ($app:expr) => {
        $crate::ops::refresh($app)
    };
}",
    expected: &["crate::ops::refresh"],
};

pub(super) const BLOCK_SCOPED_IMPORT: Case = Case {
    name: "block_scoped_import",
    file: "src/ui/status.rs",
    source: r#"fn load() -> Vec<u8> {
    use std::fs;
    fs::read("x").unwrap_or_default()
}"#,
    expected: &["std::fs", "std::fs::read"],
};

pub(super) const GLOB_IMPORT: Case = Case {
    name: "glob_import",
    file: "src/ui/status.rs",
    source: "use crate::ops::*;",
    expected: &["crate::ops::*"],
};

pub(super) const STRING_HIDES_CODE: Case = Case {
    name: "string_hides_code",
    file: "src/ui/status.rs",
    source: r#"fn wipe() {
    let _ = ("//", std::fs::remove_file("x"));
}"#,
    expected: &["std::fs::remove_file"],
};
