use super::*;

#[test]
fn a_directory_removed_before_globbing_is_ignored() {
    let root = tempfile::tempdir().unwrap();
    let compiled = CompiledGlob::compile("**").unwrap();
    let mut scan = GlobScan {
        pattern: &compiled,
        max_depth: None,
        max_results: 10,
        rel_buf: String::new(),
        results: Vec::new(),
        visited: 0,
    };
    glob_recursive(&root.path().join("gone"), 0, &mut scan);
    assert_eq!(scan.results, Vec::<serde_json::Value>::new());
}

#[test]
fn glob_walk_skips_directory_links_across_platforms() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret.txt"), b"x").unwrap();
    crate::tests_tools::make_dir_link_cross(outside.path(), &root.path().join("linked"));

    let compiled = CompiledGlob::compile("**").unwrap();
    let mut scan = GlobScan {
        pattern: &compiled,
        max_depth: None,
        max_results: 10,
        rel_buf: String::new(),
        results: Vec::new(),
        visited: 0,
    };
    glob_recursive(root.path(), 0, &mut scan);
    assert_eq!(scan.results, Vec::<serde_json::Value>::new());
}
