use super::*;

#[test]
fn a_directory_removed_before_globbing_is_ignored() {
    let root = tempfile::tempdir().unwrap();
    let scan = GlobScan { root: root.path(), pattern: "**", max_depth: None, max_results: 10 };
    let mut results = Vec::new();
    glob_recursive(&root.path().join("gone"), 0, &scan, &mut results, &mut 0);
    assert!(results.is_empty());
}
