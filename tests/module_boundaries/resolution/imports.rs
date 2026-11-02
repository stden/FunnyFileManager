//! Purpose: Use declarations: grouped, nested and renamed imports of `std` and the crate.

use super::Case;

pub(super) const GROUPED_STD_IMPORT: Case = Case {
    name: "grouped_std_import",
    file: "src/ui/status.rs",
    source: "use std::{io, fs};",
    expected: &["std::fs"],
};

pub(super) const NESTED_GROUP_IMPORT: Case = Case {
    name: "nested_group_import",
    file: "src/ui/status.rs",
    source: "use std::{io::{self, Write}, fs::File};",
    expected: &["std::fs::File"],
};

pub(super) const GROUPED_CRATE_IMPORT: Case = Case {
    name: "grouped_crate_import",
    file: "src/ui/status.rs",
    source: "use crate::{models, ops::file};",
    expected: &["crate::ops::file"],
};

pub(super) const RENAMED_STD_MODULE: Case = Case {
    name: "renamed_std_module",
    file: "src/ui/status.rs",
    source: r#"use std::fs as f;

fn load() -> Vec<u8> {
    f::read("x").unwrap_or_default()
}"#,
    expected: &["std::fs", "std::fs::read"],
};

pub(super) const RENAMED_CRATE_MODULE: Case = Case {
    name: "renamed_crate_module",
    file: "src/ui/status.rs",
    source: "use crate::ops as o;

fn copy(app: &mut App) {
    let _ = o::handle_copy(app);
}",
    expected: &["crate::ops", "crate::ops::handle_copy"],
};

pub(super) const RENAMED_CRATE_ROOT: Case = Case {
    name: "renamed_crate_root",
    file: "src/ui/status.rs",
    source: "use crate as c;

fn refresh(app: &mut App) {
    c::ops::refresh(app);
}",
    expected: &["crate::ops::refresh"],
};
