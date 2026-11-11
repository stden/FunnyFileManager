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

#[test]
fn non_existent_path_ending_in_dot_component_is_rejected_without_new_parents() {
    let root = tempfile::tempdir().unwrap();
    let canon = super::root_of(&root);
    let err = super::super::sanitize_path(&canon, "missing/.").unwrap_err();
    assert!(err.contains("invalid path"), "got: {err}");
    let parent_err = super::super::sanitize_path(&canon, "missing1/missing2/..").unwrap_err();
    assert!(parent_err.contains("invalid path"), "got: {parent_err}");
}
