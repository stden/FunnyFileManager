//! Unit tests: an edit of the file's own mapping — what it writes, and every
//! reason it writes nothing.

use super::*;

/// An edit that sets `counter` to `value`, whatever the file holds.
fn set_counter(value: Value) -> impl FnOnce(&mut Mapping, &Counter) -> io::Result<()> {
    move |document, _| {
        document.insert("counter".into(), value);
        Ok(())
    }
}

#[test]
fn a_missing_file_gets_only_the_change_under_the_header() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    let saved: Counter = edit(&path, set_counter(Value::from(7))).expect("saved");
    assert_eq!(saved, Counter { counter: 7 });
    assert_eq!(
        fs::read_to_string(&path).expect("written"),
        format!("{HEADER}counter: 7\n")
    );
}

/// The edit sees the file's settings and changes only what it touches.
#[test]
fn an_edit_keeps_every_key_it_does_not_touch_in_place() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    write_file(&path, "first: 1\ncounter: 2\nlast: [a]\n");
    let saved: Counter = edit(&path, |document, on_disk: &Counter| {
        assert_eq!(on_disk, &Counter { counter: 2 });
        document.insert("counter".into(), Value::from(on_disk.counter + 1));
        Ok(())
    })
    .expect("saved");
    assert_eq!(saved, Counter { counter: 3 });
    let document = stored(&path).document;
    let keys: Vec<_> = document.keys().filter_map(Value::as_str).collect();
    assert_eq!(keys, ["first", "counter", "last"]);
    assert_eq!(document.get("counter"), Some(&Value::from(3)));
}

#[test]
fn an_edit_that_changes_nothing_writes_nothing() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    let saved: Counter = edit(&path, |_, _| Ok(())).expect("nothing to do");
    assert_eq!(saved, Counter::default());
    assert!(!path.exists(), "no file for an empty change");

    let text = "# mine\ncounter: 5\n";
    write_file(&path, text);
    edit(&path, set_counter(Value::from(5)))
        .map(|_: Counter| ())
        .expect("same value");
    assert_eq!(
        fs::read_to_string(&path).expect("kept"),
        text,
        "comments stay"
    );
}

#[test]
fn a_refused_edit_leaves_the_file_as_it_was() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    write_file(&path, "counter: 1\n");
    let error = edit(&path, |document, _: &Counter| {
        document.insert("counter".into(), Value::from(2));
        Err(io::Error::other("the servers changed"))
    })
    .expect_err("refused");
    assert_eq!(error.to_string(), "Not saved: the servers changed");
    assert_eq!(fs::read_to_string(&path).expect("kept"), "counter: 1\n");
}

/// A writer bug is caught before the file is touched: what the edit made
/// must parse as the writer's own settings.
#[test]
fn an_edit_that_would_not_parse_is_refused_before_the_write() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    write_file(&path, "counter: 1\n");
    let error = edit(&path, set_counter(Value::from("many")))
        .map(|_: Counter| ())
        .expect_err("refused");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(
        error
            .to_string()
            .starts_with("Not saved: the change would not parse: counter: invalid type"),
        "{error}"
    );
    assert_eq!(fs::read_to_string(&path).expect("kept"), "counter: 1\n");
}

#[test]
fn an_edit_never_rewrites_a_file_that_does_not_parse() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    for text in ["counter: [\n", "counter: many\n"] {
        write_file(&path, text);
        let error = edit(&path, set_counter(Value::from(2)))
            .map(|_: Counter| ())
            .expect_err("refused");
        assert!(
            error
                .to_string()
                .starts_with("Not saved: settings.yaml does not parse: "),
            "{error}"
        );
        assert_eq!(fs::read_to_string(&path).expect("kept"), text);
    }
}

#[test]
fn an_edit_reads_the_legacy_json_and_writes_the_yaml_beside_it() {
    let temp = TempDir::new().expect("tempdir");
    let json = r#"{"counter": 3, "Editor": "8"}"#;
    write_file(&legacy_path(&temp), json);
    let path = settings_path(&temp);
    let saved: Counter = edit(&path, |document, on_disk: &Counter| {
        assert_eq!(on_disk, &Counter { counter: 3 });
        document.insert("counter".into(), Value::from(4));
        Ok(())
    })
    .expect("saved");
    assert_eq!(saved, Counter { counter: 4 });
    let expected: Mapping =
        serde_norway::from_str("counter: 4\noptions: {Editor: '8'}\n").expect("YAML");
    assert_eq!(stored(&path).document, expected);
    assert_eq!(fs::read_to_string(legacy_path(&temp)).expect("kept"), json);
}

#[test]
fn an_edit_is_refused_while_another_ffm_holds_the_lock() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    write_file(&path, "counter: 1\n");
    let lock = File::create(temp.path().join("ffm").join("settings.yaml.lock")).expect("lock file");
    lock.lock().expect("held by the other FFM");
    let error = edit(&path, set_counter(Value::from(2)))
        .map(|_: Counter| ())
        .expect_err("locked");
    assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
    assert_eq!(
        error.to_string(),
        "Not saved: another FFM is saving settings.yaml"
    );
    assert_eq!(fs::read_to_string(&path).expect("kept"), "counter: 1\n");
}

/// Edit `counter` to 2 at `path` while another program writes `text` to
/// `other`; the edit's refusal.
fn edit_while_written(path: &Path, other: &Path, text: &str) -> io::Error {
    edit(path, |document, _: &Counter| {
        write_file(other, text);
        document.insert("counter".into(), Value::from(2));
        Ok(())
    })
    .map(|_: Counter| ())
    .expect_err("refused")
}

/// Only FFMs take the lock: what an editor, `sed -i` or a dotfiles sync writes
/// while a save is under way is kept, whether the save read the YAML, the
/// legacy JSON or no file at all.
#[test]
fn a_file_changed_while_saving_keeps_the_other_programs_change() {
    let yaml: FixturePath = settings_path;
    let json: FixturePath = legacy_path;
    let cases = [
        (Some("counter: 1\n"), yaml, "counter: 9\n"),
        (Some(r#"{"counter": 1}"#), json, r#"{"counter": 9}"#),
        (None, yaml, "counter: 9\n"),
    ];
    for (before, fixture, meanwhile) in cases {
        let temp = TempDir::new().expect("tempdir");
        let path = settings_path(&temp);
        let other = fixture(&temp);
        if let Some(text) = before {
            write_file(&other, text);
        }
        let error = edit_while_written(&path, &other, meanwhile);
        assert_eq!(
            error.to_string(),
            "Not saved: settings.yaml changed while it was being saved; save again"
        );
        assert_eq!(fs::read_to_string(&other).expect("kept"), meanwhile);
        assert_eq!(
            stored(&path).parse::<Counter>().expect("parses"),
            Counter { counter: 9 },
            "{meanwhile}"
        );
    }
}
