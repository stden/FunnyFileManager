//! Unit tests: a save refuses to replace a stored file it cannot read or
//! that does not parse, and says why.

use super::*;

/// Hand edits that break a file: a syntax error, a duplicate key, a tab
/// indent, a tag, and a broken legacy file.
#[test]
fn a_save_never_overwrites_a_file_that_does_not_parse() {
    let broken: [(FixturePath, &str); 6] = [
        (settings_path, "counter: [\n"),
        (settings_path, "counter: 1\ncounter: 2\n"),
        (settings_path, "custom:\n\tnested: 1\n"),
        (settings_path, "custom: !x 1\n"),
        (legacy_path, "{ not json"),
        (legacy_path, r#"{"counter": 1, "counter": 2}"#),
    ];
    for (file, text) in broken {
        let temp = TempDir::new().expect("tempdir");
        write_file(&file(&temp), text);
        let error =
            save(&settings_path(&temp), 9).expect_err("a file that does not parse is not replaced");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        let message = error.to_string();
        assert!(message.contains("does not parse"), "{message}");
        assert!(message.starts_with(NOT_SAVED), "{message}");
        assert_eq!(fs::read_to_string(file(&temp)).expect("kept"), text);
        if file(&temp) != settings_path(&temp) {
            assert!(!settings_path(&temp).exists(), "no YAML shadows it");
        }
    }
}

#[test]
fn a_save_never_overwrites_a_file_whose_own_keys_do_not_parse() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    let text = "counter: many\noptions: {Editor: '8'}\n";
    write_file(&path, text);
    let error = save(&path, 1).expect_err("refused");
    assert!(
        error
            .to_string()
            .starts_with("Not saved: settings.yaml does not parse: counter: "),
        "{error}"
    );
    assert_eq!(fs::read_to_string(&path).expect("kept"), text);
}

#[test]
fn a_save_that_cannot_read_the_stored_file_writes_nothing() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    fs::create_dir_all(&path).expect("a directory where the file goes");
    let error = save(&path, 1).expect_err("refused");
    assert_eq!(
        error.to_string(),
        "Not saved: settings.yaml is not a regular file"
    );
    assert!(path.is_dir());
}

#[test]
fn a_save_that_cannot_create_the_config_directory_says_which() {
    let temp = TempDir::new().expect("tempdir");
    let blocked = temp.path().join("ffm");
    fs::write(&blocked, "").expect("a file where the directory goes");
    let error = save(&blocked.join(FILE_NAME), 1).expect_err("refused");
    let message = error.to_string();
    assert!(
        message.starts_with(&format!("{NOT_SAVED}cannot create folder ffm: ")),
        "{message}"
    );
}

/// A settings file this user may not write (0444, as a guest) was refused
/// as `Not saved: /tmp/…/.config/ffm/settings.yaml: permission denied`,
/// and at 120 columns the reason fell off the end of the status line. Root
/// writes past the mode bits, so there it is saved.
#[cfg(unix)]
#[test]
fn a_read_only_settings_file_is_refused_by_its_name() {
    use std::os::unix::fs::PermissionsExt as _;

    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    write_file(&path, "counter: 1\n");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).expect("chmod");
    let result = update(&path, &Counter { counter: 1 }, &Counter { counter: 2 });
    if crate::platform::is_root() {
        result.expect("root writes past the mode bits");
        return;
    }
    let error = result.expect_err("refused");
    assert_eq!(
        error.to_string(),
        "Not saved: settings.yaml: permission denied"
    );
}

#[test]
fn only_a_mapping_can_be_saved() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    let error = update(&path, &Vec::new(), &vec![1_u32]).expect_err("a list has no keys");
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert!(!path.exists());
}

/// A FIFO where the lock goes: opening it for writing waited for a reader
/// that never came, and the save — on the event loop, and again at exit —
/// froze FFM with Esc and F10 dead. The lock must be a regular file, and
/// looking at one never waits.
#[cfg(unix)]
#[test]
fn a_lock_that_is_not_a_regular_file_refuses_the_save_without_waiting() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    write_file(&path, "counter: 1\n");
    let mut lock = path.clone().into_os_string();
    lock.push(LOCK_SUFFIX);
    crate::test_support::make_fifo(Path::new(&lock)).expect("a FIFO where the lock goes");
    let outcome =
        crate::test_support::run_with_timeout(std::time::Duration::from_secs(5), move || {
            save(&path, 2).map_err(|error| error.to_string())
        })
        .expect("the save must not wait on the FIFO");
    assert_eq!(
        outcome,
        Err("Not saved: settings.yaml.lock: not a regular file".to_owned())
    );
}
