use super::*;

#[test]
fn a_directory_that_disappears_produces_no_search_results() {
    let root = tempfile::tempdir().unwrap();
    let missing = root.path().join("absent");
    let scan = SearchScan { pattern: "", files_only: false, dirs_only: false, max: 10 };
    let mut results = Vec::new();
    search_recursive(&missing, &scan, &mut results, &mut 0);
    assert!(results.is_empty());
}
