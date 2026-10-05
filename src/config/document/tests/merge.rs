//! Unit tests: what a save writes — the entries the session changed — and
//! what it keeps: hand edits, other writers' keys, comments of a file it
//! does not need to change. How entries changed on both sides merge is in
//! `merge/entries.rs`; the lock and the atomic rename in `merge/atomic.rs`.

use super::*;

mod atomic;
mod entries;

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
