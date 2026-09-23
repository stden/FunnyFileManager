//! Unit tests: what a save writes — the entries the session changed — and
//! what it keeps: hand edits, other writers' keys, comments of a file it
//! does not need to change, and the file whole when both sides changed one
//! entry or another FFM is saving.

use std::collections::BTreeMap;

use super::*;

/// A stand-in writer with two pages and a list, like the typed settings.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct Pages {
    first: Page,
    second: Page,
    list: Vec<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct Page {
    a: u32,
    b: u32,
}

fn pages(first_a: u32, second_b: u32) -> Pages {
    let mut pages = Pages::default();
    pages.first.a = first_a;
    pages.second.b = second_b;
    pages
}

fn document(path: &Path) -> Mapping {
    stored(path).document
}

fn yaml(text: &str) -> Value {
    serde_norway::from_str(text).expect("YAML")
}

#[test]
fn a_first_save_writes_everything_under_the_header() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    save(&path, 5).expect("save");
    let text = fs::read_to_string(&path).expect("saved");
    assert_eq!(text, format!("{HEADER}counter: 5\n"));
    update(&path, &Pages::default(), &Pages::default()).expect("save");
    assert_eq!(
        fs::read_to_string(&path).expect("saved"),
        text,
        "a writer that changed nothing adds nothing to an existing file"
    );
}

#[test]
fn a_save_keeps_every_key_it_does_not_own_in_place() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    write_file(
        &path,
        "options: {Editor: '8'}\ncounter: 1\ncustom:\n  nested: [a, b]\n",
    );
    update(&path, &Counter { counter: 1 }, &Counter { counter: 2 }).expect("save");
    let document = document(&path);
    let keys: Vec<_> = document.keys().filter_map(Value::as_str).collect();
    assert_eq!(keys, ["options", "counter", "custom"], "the order is kept");
    assert_eq!(document.get("options"), Some(&yaml("{Editor: '8'}")));
    assert_eq!(document.get("counter"), Some(&Value::from(2)));
    assert_eq!(document.get("custom"), Some(&yaml("nested: [a, b]")));
}

#[test]
fn the_first_save_migrates_the_legacy_json_and_leaves_it_alone() {
    let temp = TempDir::new().expect("tempdir");
    let json = r#"{"Editor": "8", "counter": 1}"#;
    write_file(&legacy_path(&temp), json);
    let loaded = Counter { counter: 1 };
    update(&settings_path(&temp), &loaded, &loaded).expect("save");
    assert_eq!(fs::read_to_string(legacy_path(&temp)).expect("kept"), json);
    let stored = stored(&settings_path(&temp));
    assert_eq!(
        stored.format,
        Format::Yaml,
        "written though nothing changed"
    );
    assert_eq!(stored.document.get("counter"), Some(&Value::from(1)));
    assert_eq!(stored.document.get("options"), Some(&yaml("{Editor: '8'}")));
}

/// Save setup and the exit auto-save run with nothing changed most of the
/// time; they must not strip the comments of a file edited by hand.
#[test]
fn a_save_that_changes_nothing_leaves_the_file_alone() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    let text = "# mine\nfirst: {a: 7}   # hand-tuned\nsecond:\n  b: 0x10\n";
    write_file(&path, text);
    let loaded = pages(7, 16);
    let mut current = loaded.clone();
    update(&path, &loaded, &current).expect("save");
    assert_eq!(fs::read_to_string(&path).expect("kept"), text);
    current.second.b = 16;
    current.first.b = 0;
    update(&path, &Pages::default(), &current).expect("the file holds it already");
    assert_eq!(fs::read_to_string(&path).expect("kept"), text);
}

#[test]
fn a_hand_edit_made_while_ffm_runs_survives_a_save() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    write_file(&path, "first: {a: 1}\n");
    let loaded = pages(1, 0);
    write_file(&path, "first: {a: 5}\n");
    update(&path, &loaded, &pages(1, 3)).expect("save");
    assert_eq!(
        document(&path),
        yaml("first: {a: 5}\nsecond: {a: 0, b: 3}")
            .as_mapping()
            .cloned()
            .expect("mapping")
    );
}

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

/// Two FFMs saving at once: the second waits for no one and says why.
#[test]
fn a_save_is_refused_while_another_ffm_holds_the_lock() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    write_file(&path, "counter: 1\n");
    let lock =
        fs::File::create(temp.path().join("ffm").join("settings.yaml.lock")).expect("lock file");
    lock.lock().expect("held by the other FFM");
    let loaded = Counter { counter: 1 };
    let current = Counter { counter: 2 };
    let error = update(&path, &loaded, &current).expect_err("locked");
    assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
    assert_eq!(
        error.to_string(),
        "Not saved: another FFM is saving settings.yaml"
    );
    assert_eq!(fs::read_to_string(&path).expect("kept"), "counter: 1\n");
    drop(lock);
    update(&path, &loaded, &current).expect("free again");
    assert_eq!(document(&path).get("counter"), Some(&Value::from(2)));
}

#[test]
fn a_lock_that_cannot_be_opened_refuses_the_save() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    fs::create_dir_all(temp.path().join("ffm").join("settings.yaml.lock"))
        .expect("a directory where the lock goes");
    let error = save(&path, 2).expect_err("no lock");
    assert!(
        error
            .to_string()
            .starts_with("Not saved: settings.yaml.lock: "),
        "{error}"
    );
    assert!(!path.exists());
}

#[cfg(unix)]
#[test]
fn a_save_renames_a_private_temp_file_over_the_old_one() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    write_file(&path, "counter: 1\n");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("chmod");
    let before = fs::metadata(&path).expect("stat").ino();
    update(&path, &Counter { counter: 1 }, &Counter { counter: 2 }).expect("save");
    let after = fs::metadata(&path).expect("stat");
    assert_ne!(after.ino(), before, "a new file replaced the old one");
    assert_eq!(after.permissions().mode() & 0o777, 0o600);
    let mut names: Vec<_> = fs::read_dir(temp.path().join("ffm"))
        .expect("config dir")
        .map(|entry| entry.expect("entry").file_name())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [FILE_NAME, "settings.yaml.lock"],
        "no temp file is left behind"
    );
}
