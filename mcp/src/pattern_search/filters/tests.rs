use super::*;
use std::time::SystemTime;

fn params() -> FindParams {
    serde_json::from_value(json!({"path":"."})).unwrap()
}

#[test]
fn exhausted_limits_and_missing_directories_stop_find_without_results() {
    let root = tempfile::tempdir().unwrap();
    let now = SystemTime::now();
    let mut results = Vec::new();
    let mut options = params();
    options.max_results = 0;
    find_recursive(root.path(), 0, &options, now, &mut results, &mut 0);
    options.max_results = 1;
    find_recursive(&root.path().join("missing"), 0, &options, now, &mut results, &mut 0);
    fs::write(root.path().join("one"), b"one").unwrap();
    fs::write(root.path().join("two"), b"two").unwrap();
    find_recursive(root.path(), 0, &options, now, &mut results, &mut 0);
    assert_eq!(results.len(), 1);
}

#[test]
fn unavailable_and_future_modification_times_remain_eligible() {
    let now = SystemTime::now();
    assert!(modified_within(&Err(std::io::Error::from(std::io::ErrorKind::Unsupported)), now, 0));
    assert!(modified_within(&Ok(now + std::time::Duration::from_secs(60)), now, 0));
    assert!(!modified_within(&Ok(SystemTime::UNIX_EPOCH), now, 1));
}

#[test]
fn an_entry_removed_after_enumeration_is_never_reported() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("gone");
    fs::write(&path, b"gone").unwrap();
    let entry = fs::read_dir(root.path()).unwrap().next().unwrap().unwrap();
    fs::remove_file(path).unwrap();
    let mut results = Vec::new();
    visit_find_entry(&entry, 0, &params(), SystemTime::now(), &mut results, &mut 0);
    assert!(results.is_empty());
}
