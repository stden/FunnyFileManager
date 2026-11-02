use std::path::Path;

#[test]
fn a_path_without_any_existing_ancestor_reports_the_resolution_failure() {
    let root = tempfile::tempdir().unwrap();
    let error =
        super::super::resolve_under_existing_ancestor(root.path(), Path::new("")).unwrap_err();
    assert!(error.contains("no existing ancestor directory"));
}

#[test]
fn current_directory_components_are_folded_before_new_parent_resolution() {
    assert_eq!(
        super::super::resolve_dot_dot(Path::new("./missing")).unwrap(),
        Path::new("missing")
    );
}
