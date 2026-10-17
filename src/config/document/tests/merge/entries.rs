//! Unit tests: an entry changed in the file, in the session or in both —
//! merged key by key, kept, removed, or refused as a conflict.

use std::collections::BTreeMap;

use super::*;

#[test]
fn a_value_changed_in_the_file_and_in_the_session_refuses_the_save() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    let text = "first: {a: 5}\n";
    write_file(&path, text);
    let error = update(&path, &pages(1, 0), &pages(2, 0)).expect_err("a conflict");
    assert_eq!(
        error.to_string(),
        "Not saved: settings.yaml changed on disk: first.a was edited there and in this session"
    );
    assert_eq!(fs::read_to_string(&path).expect("kept"), text);
}

#[test]
fn changes_to_different_keys_of_one_page_merge() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    write_file(&path, "first: {a: 5}\nlist: [x]\n");
    let loaded = pages(1, 0);
    let mut current = loaded.clone();
    current.first.b = 9;
    update(&path, &loaded, &current).expect("save");
    assert_eq!(
        document(&path),
        yaml("first: {a: 5, b: 9}\nlist: [x]")
            .as_mapping()
            .cloned()
            .expect("mapping")
    );
}

#[test]
fn the_same_change_in_the_file_and_the_session_is_no_conflict() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    let text = "first: {a: 2}\n";
    write_file(&path, text);
    update(&path, &pages(1, 0), &pages(2, 0)).expect("save");
    assert_eq!(fs::read_to_string(&path).expect("kept"), text);
}

#[test]
fn an_entry_that_is_no_mapping_on_one_side_conflicts_as_a_whole() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    write_file(&path, "first: ~\n");
    let loaded = pages(1, 0);
    let mut current = loaded.clone();
    current.first.b = 2;
    let error = update(&path, &loaded, &current).expect_err("first is null in the file");
    assert!(error.to_string().contains(": first was edited"), "{error}");

    write_file(&path, "list: [y]\n");
    let loaded = Pages {
        list: vec!["x".to_owned()],
        ..Pages::default()
    };
    let mut current = loaded.clone();
    current.list.push("z".to_owned());
    let error = update(&path, &loaded, &current).expect_err("a list is one value");
    assert!(error.to_string().contains(": list was edited"), "{error}");
    assert_eq!(fs::read_to_string(&path).expect("kept"), "list: [y]\n");
}

#[test]
fn an_entry_removed_in_the_session_is_removed_unless_the_file_changed_it() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    write_file(&path, "x: 1\ny: 2\nz: 3\n");
    let loaded = BTreeMap::from([("x", 1), ("y", 2), ("z", 3)].map(|(k, v)| (k.to_owned(), v)));
    let mut current = loaded.clone();
    current.remove("y");
    update(&path, &loaded, &current).expect("save");
    let keys: Vec<_> = document(&path)
        .keys()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    assert_eq!(keys, ["x", "z"], "the others keep their order");

    write_file(&path, "x: 1\ny: 4\n");
    let error = update(&path, &loaded, &current).expect_err("y was edited");
    assert!(error.to_string().contains(": y was edited"), "{error}");
}

#[test]
fn a_page_the_file_leaves_out_is_written_whole_and_a_partial_one_stays_partial() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    write_file(&path, "first: {a: 5}\n");
    let loaded = pages(5, 0);
    update(&path, &loaded, &pages(5, 3)).expect("save");
    let text = fs::read_to_string(&path).expect("saved");
    assert!(
        text.contains("\nfirst:\n  a: 5\nsecond:\n  a: 0\n  b: 3\n"),
        "{text}"
    );
}
