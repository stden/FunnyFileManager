use super::*;

fn params() -> FindParams {
    serde_json::from_value(json!({"path":"."})).unwrap()
}

#[test]
fn exhausted_limits_and_missing_directories_stop_find_without_results() {
    let root = tempfile::tempdir().unwrap();
    let now = SystemTime::now();
    let mut options = params();
    options.max_results = 0;
    let mut run = FindRun {
        params: &options,
        name_glob: None,
        now,
        results: Vec::new(),
        visited: 0,
    };
    find_recursive(root.path(), 0, &mut run);
    assert_eq!(run.results, Vec::<serde_json::Value>::new());

    let mut one_opt = params();
    one_opt.max_results = 1;
    let mut run2 = FindRun {
        params: &one_opt,
        name_glob: None,
        now,
        results: Vec::new(),
        visited: 0,
    };
    find_recursive(&root.path().join("missing"), 0, &mut run2);
    fs::write(root.path().join("one"), b"one").unwrap();
    fs::write(root.path().join("two"), b"two").unwrap();
    find_recursive(root.path(), 0, &mut run2);
    assert_eq!(run2.results.len(), 1);
}

#[test]
fn unavailable_and_future_modification_times_remain_eligible() {
    let now = SystemTime::now();
    assert!(modified_within(
        &Err(std::io::Error::from(std::io::ErrorKind::Unsupported)),
        now,
        0
    ));
    assert!(modified_within(
        &Ok(now + std::time::Duration::from_secs(60)),
        now,
        0
    ));
    assert!(!modified_within(&Ok(SystemTime::UNIX_EPOCH), now, 1));
}

#[test]
fn an_entry_removed_after_enumeration_is_never_reported() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("gone");
    fs::write(&path, b"gone").unwrap();
    let entry = fs::read_dir(root.path()).unwrap().next().unwrap().unwrap();
    fs::remove_file(path).unwrap();
    let options = params();
    let mut run = FindRun {
        params: &options,
        name_glob: None,
        now: SystemTime::now(),
        results: Vec::new(),
        visited: 0,
    };
    visit_find_entry(&entry, 0, &mut run);
    assert_eq!(run.results, Vec::<serde_json::Value>::new());
}
