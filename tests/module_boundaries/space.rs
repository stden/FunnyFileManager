//! Purpose: Space analysis only suggests. Its verdicts, facts and dialog import
//! no filesystem or process API and reach none of the deleting operations, and
//! the shared walk it measures with reaches no removal, rename or write.

use super::architecture::{forbidden_dependencies, SourceFile};
use super::{messages, tui_sources};

/// The feature: the pure verdict, its facts, its state, keys and rendering.
fn is_feature(file: &SourceFile) -> bool {
    let feature_paths = [
        "src/space_verdict",
        "src/ops/space_analysis",
        "src/models/space_analysis",
        "src/popup/space_analysis",
        "src/ui/space_analysis",
    ];
    !file.is_test()
        && feature_paths
            .iter()
            .any(|path| file.relative.starts_with(path))
}

/// Filesystem and process access the feature never needs: it measures through
/// `stats`, asks git through the runner in `ops::git_pull`, and never removes.
const FEATURE_FORBIDDEN: [&str; 7] = [
    "std::fs",
    "std::process",
    "std::net",
    "crate::ops::file",
    "crate::ops::disk_cleanup",
    "crate::ops::wipe",
    "crate::ops::apply",
];

/// Calls that remove, rename or write a path. The shared walk may read only.
const WALK_FORBIDDEN: [&str; 10] = [
    "std::fs::remove_file",
    "std::fs::remove_dir",
    "std::fs::remove_dir_all",
    "std::fs::rename",
    "std::fs::write",
    "std::fs::copy",
    "std::fs::create_dir",
    "std::fs::create_dir_all",
    "std::fs::hard_link",
    "std::os::unix::fs::symlink",
];

fn feature_violations(sources: &[SourceFile]) -> Vec<String> {
    messages(&forbidden_dependencies(
        sources,
        is_feature,
        &FEATURE_FORBIDDEN,
    ))
}

fn walk_violations(sources: &[SourceFile]) -> Vec<String> {
    let checked = |file: &SourceFile| !file.is_test() && file.in_module("stats");
    messages(&forbidden_dependencies(sources, checked, &WALK_FORBIDDEN))
}

#[test]
fn space_analysis_imports_no_filesystem_process_or_deleting_operation() {
    assert_eq!(feature_violations(&tui_sources()), Vec::<String>::new());
}

#[test]
fn the_shared_walk_reaches_no_removal_rename_or_write() {
    assert_eq!(walk_violations(&tui_sources()), Vec::<String>::new());
}

#[test]
fn a_planted_removal_in_the_feature_is_reported() {
    let sources = [SourceFile::new(
        "src/ops/space_analysis/planted.rs",
        "use std::fs;\n\nfn go(path: &std::path::Path) {\n    let _ = fs::remove_file(path);\n}\n",
    )];
    let violations = feature_violations(&sources);
    assert!(
        violations
            .iter()
            .any(|message| message.contains("std::fs::remove_file")),
        "{violations:?}"
    );
}

#[test]
fn reading_the_disk_through_the_walk_is_allowed_in_the_feature() {
    let sources = [SourceFile::new(
        "src/ops/space_analysis/clean.rs",
        "use std::io;\n\nfn measure(root: &crate::stats::Budget) -> io::Result<()> {\n    let _ = root;\n    Ok(())\n}\n",
    )];
    assert_eq!(feature_violations(&sources), Vec::<String>::new());
}

#[test]
fn a_planted_rename_in_the_walk_is_reported() {
    let sources = [SourceFile::new(
        "src/stats/planted.rs",
        "fn move_it(from: &str, to: &str) {\n    let _ = std::fs::rename(from, to);\n}\n",
    )];
    let violations = walk_violations(&sources);
    assert!(
        violations
            .iter()
            .any(|message| message.contains("std::fs::rename")),
        "{violations:?}"
    );
}

#[test]
fn a_reach_into_the_deleting_operations_is_reported() {
    let sources = [SourceFile::new(
        "src/ops/space_analysis.rs",
        "use crate::ops::disk_cleanup::run;\n",
    )];
    assert_eq!(
        feature_violations(&sources),
        ["src/ops/space_analysis.rs:1: `crate::ops::disk_cleanup::run` (forbidden: `crate::ops::disk_cleanup`)"]
    );
}
