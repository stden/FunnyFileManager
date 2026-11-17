use super::*;

#[test]
fn a_directory_removed_before_globbing_is_ignored() {
    let root = tempfile::tempdir().unwrap();
    let compiled = CompiledGlob::compile("**").unwrap();
    let mut scan = GlobScan {
        pattern: &compiled,
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
        max_results: 10,
        rel_buf: String::new(),
        results: Vec::new(),
        visited: 0,
    };
    glob_recursive(root.path(), 0, &mut scan);
    assert_eq!(scan.results, Vec::<serde_json::Value>::new());
}

#[test]
fn glob_prunes_non_matching_leading_subtrees_before_double_star() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("src/sub")).unwrap();
    fs::write(root.path().join("src/sub/lib.rs"), b"pub fn f() {}").unwrap();
    fs::create_dir_all(root.path().join("target/debug")).unwrap();
    for idx in 0..20 {
        fs::write(root.path().join(format!("target/debug/f{idx}.rs")), b"x").unwrap();
    }

    let compiled = CompiledGlob::compile("src/**/*.rs").unwrap();
    let mut scan = GlobScan {
        pattern: &compiled,
        max_results: 50,
        rel_buf: String::new(),
        results: Vec::new(),
        visited: 0,
    };
    glob_recursive(root.path(), 0, &mut scan);
    assert_eq!(scan.results.len(), 1);
    assert_eq!(scan.visited, 4, "target/ subtree must be pruned at depth 0");
}
