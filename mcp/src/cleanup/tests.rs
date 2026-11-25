use super::*;

fn params() -> DeleteByPatternParams {
    serde_json::from_value(json!({"path":".","pattern":"*","recursive":false,"include_dirs":true}))
        .unwrap()
}

#[test]
fn a_nonrecursive_delete_removes_empty_directories_but_reports_nonempty_ones() {
    let root = tempfile::tempdir().unwrap();
    let empty = root.path().join("empty");
    let full = root.path().join("full");
    fs::create_dir(&empty).unwrap();
    fs::create_dir(&full).unwrap();
    fs::write(full.join("keep"), b"preserved").unwrap();
    let compiled = CompiledGlob::compile("*").unwrap();
    let mut files_only = params();
    files_only.include_dirs = false;
    assert_eq!(
        delete_pattern_recursive(
            root.path(),
            &compiled,
            &files_only,
            &mut DeleteRun::default()
        )
        .claimed,
        0
    );
    let mut run = DeleteRun::default();
    delete_pattern_recursive(root.path(), &compiled, &params(), &mut run);
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
    let compiled = CompiledGlob::compile("*").unwrap();
    let mut run = DeleteRun {
        visited: WALK_BUDGET,
        ..DeleteRun::default()
    };
    assert_eq!(
        delete_pattern_recursive(root.path(), &compiled, &params(), &mut run).claimed,
        0
    );
    run.visited = 0;
    assert_eq!(
        delete_pattern_recursive(&root.path().join("missing"), &compiled, &params(), &mut run)
            .claimed,
        0
    );
    run.visited = WALK_BUDGET - 1;
    let partial = delete_pattern_recursive(root.path(), &compiled, &params(), &mut run);
    assert!(!partial.complete);
    assert_eq!(run.deleted.len(), 1);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn recursive_nested_directory_matches_accumulate_sizes_without_double_counting() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let outer = root.path().join("cache_outer");
    let inner = outer.join("cache_inner");
    fs::create_dir_all(&inner).unwrap();
    fs::write(inner.join("blob.bin"), vec![b'x'; 120]).unwrap();
    fs::write(outer.join("keep.txt"), vec![b'y'; 30]).unwrap();
    crate::tests_tools::make_dir_link_cross(outside.path(), &root.path().join("cache_link"));

    let compiled = CompiledGlob::compile("cache*").unwrap();
    let dry: DeleteByPatternParams = serde_json::from_value(
        json!({"path":".","pattern":"cache*","recursive":true,"include_dirs":true,"dry_run":true}),
    )
    .unwrap();
    let mut dry_run = DeleteRun::default();
    let dry_totals = delete_pattern_recursive(root.path(), &compiled, &dry, &mut dry_run);
    assert_eq!(dry_totals.claimed, 150);

    let live: DeleteByPatternParams = serde_json::from_value(
        json!({"path":".","pattern":"cache*","recursive":true,"include_dirs":true,"dry_run":false}),
    )
    .unwrap();
    let mut live_run = DeleteRun::default();
    let live_totals = delete_pattern_recursive(root.path(), &compiled, &live, &mut live_run);
    assert_eq!(live_totals.claimed, 150);
    assert!(!outer.exists());
}
