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
        message.starts_with(&format!("{NOT_SAVED}{}: ", blocked.display())),
        "{message}"
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
