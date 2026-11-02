use super::*;

fn params() -> DeleteByPatternParams {
    serde_json::from_value(json!({"path":".","pattern":"*","recursive":false,"include_dirs":true})).unwrap()
}

#[test]
fn a_nonrecursive_delete_removes_empty_directories_but_reports_nonempty_ones() {
    let root = tempfile::tempdir().unwrap();
    let empty = root.path().join("empty");
    let full = root.path().join("full");
    fs::create_dir(&empty).unwrap();
    fs::create_dir(&full).unwrap();
    fs::write(full.join("keep"), b"preserved").unwrap();
    let mut run = DeleteRun::default();
    delete_pattern_recursive(root.path(), &params(), &mut run);
    assert!(!empty.exists());
    assert_eq!(fs::read(full.join("keep")).unwrap(), b"preserved");
    assert_eq!(run.deleted.len(), 1);
    assert_eq!(run.errors.len(), 1);
    assert_eq!(run.errors[0]["path"], full.display().to_string());
}

#[test]
fn missing_directories_and_exhausted_walk_budgets_do_not_delete_entries() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("one"), b"one").unwrap();
    fs::write(root.path().join("two"), b"two").unwrap();
    let mut run = DeleteRun { visited: WALK_BUDGET, ..DeleteRun::default() };
    assert_eq!(delete_pattern_recursive(root.path(), &params(), &mut run), 0);
    run.visited = 0;
    assert_eq!(delete_pattern_recursive(&root.path().join("missing"), &params(), &mut run), 0);
    run.visited = WALK_BUDGET - 1;
    delete_pattern_recursive(root.path(), &params(), &mut run);
    assert_eq!(run.deleted.len(), 1);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}
