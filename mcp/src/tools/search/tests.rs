use super::*;

#[test]
fn a_directory_that_disappears_produces_no_search_results() {
    let root = tempfile::tempdir().unwrap();
    let missing = root.path().join("absent");
    let scan = SearchScan {
        pattern: "",
        files_only: false,
        dirs_only: false,
        max: 10,
    };
    let mut results = Vec::new();
    search_recursive(&missing, &scan, &mut results, &mut 0);
    assert_eq!(results, Vec::<serde_json::Value>::new());
}

#[test]
fn search_walk_skips_directory_links_across_platforms() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("needle.txt"), b"x").unwrap();
    crate::tests_tools::make_dir_link_cross(outside.path(), &root.path().join("needle_link"));

    let scan = SearchScan {
        pattern: "needle",
        files_only: false,
        dirs_only: false,
        max: 10,
    };
    let mut results = Vec::new();
    search_recursive(root.path(), &scan, &mut results, &mut 0);
    assert_eq!(results, Vec::<serde_json::Value>::new());
}
