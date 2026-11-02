//! Purpose: Scanning rejects byte-count overflow and tolerates failed entry reads.

use super::*;

fn plan(root: &Path) -> TempPlan {
    new_plan(
        &TempSource::Directory {
            path: root.to_owned(),
            home: None,
        },
        SystemTime::now(),
    )
    .unwrap()
}

#[test]
fn an_entry_error_is_counted_as_skipped_and_does_not_add_bytes() {
    let root = tempfile::TempDir::new().unwrap();
    let mut plan = plan(root.path());
    let error = fs::read_dir(root.path().join("removed")).unwrap_err();
    inspect(Err(error), &mut plan, &mut Vec::new()).unwrap();
    assert_eq!(plan.skipped, 1);
    assert_eq!(plan.bytes.bytes(), 0);
    assert!(plan.files.is_empty());
}

#[test]
fn overflowing_the_total_rejects_the_plan_without_wrapping_or_adding_the_file() {
    let root = tempfile::TempDir::new().unwrap();
    let file = root.path().join("old");
    fs::write(&file, b"x").unwrap();
    fs::File::options()
        .write(true)
        .open(&file)
        .unwrap()
        .set_modified(SystemTime::now() - std::time::Duration::from_hours(192))
        .unwrap();
    let mut plan = plan(root.path());
    plan.bytes = FileSize::from_bytes(u64::MAX);
    let entry = fs::read_dir(root.path()).unwrap().next().unwrap();
    assert!(matches!(
        inspect(entry, &mut plan, &mut Vec::new()),
        Err(TempError::Limit)
    ));
    assert_eq!(plan.bytes.bytes(), u64::MAX);
    assert!(plan.files.is_empty());
    assert_eq!(fs::read(file).unwrap(), b"x");
}

#[test]
fn the_entry_budget_stops_the_walk_without_modifying_any_files() {
    let root = tempfile::TempDir::new().unwrap();
    for name in ["a", "b", "c"] {
        fs::write(root.path().join(name), b"keep").unwrap();
    }
    assert!(matches!(
        scan_plan(plan(root.path()), 2),
        Err(TempError::Limit)
    ));
    let complete = scan_plan(plan(root.path()), 3).unwrap();
    assert_eq!(complete.skipped, 3);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 3);
    assert_eq!(fs::read(root.path().join("c")).unwrap(), b"keep");
}

#[test]
fn a_root_replaced_after_validation_is_never_walked() {
    let root = tempfile::TempDir::new().unwrap();
    let path = root.path().join("temp");
    fs::create_dir(&path).unwrap();
    let initial = plan(&path);
    fs::rename(&path, root.path().join("original")).unwrap();
    fs::create_dir(&path).unwrap();
    fs::write(path.join("keep"), b"new root").unwrap();
    let scanned = scan_plan(initial, MAX_ENTRIES).unwrap();
    assert_eq!(scanned.skipped, 1);
    assert!(scanned.files.is_empty());
    assert_eq!(scanned.bytes.bytes(), 0);
    assert_eq!(fs::read(path.join("keep")).unwrap(), b"new root");
}
