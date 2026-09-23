//! Unit tests: which file a load reads, what it reads from it, and when a
//! save refuses to touch the file. What a save writes is in `tests/merge.rs`.

use std::fs;

use serde::{Deserialize, Serialize};
use serde_norway::{Mapping, Value};
use tempfile::TempDir;

use super::*;

mod merge;

/// A stand-in owner of one top-level key, `counter`.
#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct Counter {
    counter: u32,
}

/// Where a fixture file goes in a scenario's temp dir.
type FixturePath = fn(&TempDir) -> PathBuf;

fn settings_path(temp: &TempDir) -> PathBuf {
    temp.path().join("ffm").join(FILE_NAME)
}

fn legacy_path(temp: &TempDir) -> PathBuf {
    temp.path().join("ffm").join(LEGACY_FILE_NAME)
}

fn write_file(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("config dir");
    fs::write(path, text).expect("settings fixture");
}

fn stored(path: &Path) -> Stored {
    read(path).expect("readable").expect("a stored file")
}

/// Save `counter` as a writer that loaded nothing, so holds the defaults.
fn save(path: &Path, counter: u32) -> io::Result<()> {
    update(path, &Counter::default(), &Counter { counter })
}

#[test]
fn nothing_is_stored_before_the_first_save() {
    let temp = TempDir::new().expect("tempdir");
    assert!(read(&settings_path(&temp)).expect("readable").is_none());
}

#[test]
fn the_legacy_json_is_read_while_there_is_no_yaml() {
    let temp = TempDir::new().expect("tempdir");
    write_file(&legacy_path(&temp), r#"{"counter": 3}"#);
    let stored = stored(&settings_path(&temp));
    assert_eq!(stored.format, Format::LegacyJson);
    assert_eq!(
        stored.parse::<Counter>().expect("JSON"),
        Counter { counter: 3 }
    );
}

/// The Options pages sat beside the typed keys in `settings.json`; YAML
/// keeps them under `options`, away from `editor` and the other typed keys.
#[test]
fn the_legacy_pages_are_read_under_options() {
    let temp = TempDir::new().expect("tempdir");
    write_file(
        &legacy_path(&temp),
        r#"{"Editor": "8", "editor": {"tab_size": 2}, "Panel": "off"}"#,
    );
    let document = stored(&settings_path(&temp)).document;
    let expected: Mapping =
        serde_norway::from_str("editor: {tab_size: 2}\noptions: {Editor: '8', Panel: 'off'}\n")
            .expect("YAML");
    assert_eq!(document, expected);
    let keys: Vec<_> = document.keys().filter_map(Value::as_str).collect();
    assert_eq!(keys, ["editor", "options"]);
}

#[test]
fn settings_yaml_wins_over_the_legacy_json() {
    let temp = TempDir::new().expect("tempdir");
    write_file(&legacy_path(&temp), r#"{"counter": 3}"#);
    write_file(&settings_path(&temp), "counter: 4\n");
    let stored = stored(&settings_path(&temp));
    assert_eq!(stored.format, Format::Yaml);
    assert_eq!(
        stored.parse::<Counter>().expect("YAML"),
        Counter { counter: 4 }
    );
}

#[test]
fn a_broken_settings_yaml_is_reported_not_replaced_by_the_legacy_json() {
    let temp = TempDir::new().expect("tempdir");
    write_file(&legacy_path(&temp), r#"{"counter": 3}"#);
    write_file(&settings_path(&temp), "counter: [\n");
    let error = read(&settings_path(&temp)).expect_err("broken YAML");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    let message = error.to_string();
    assert!(
        message.starts_with("settings.yaml does not parse: "),
        "{message}"
    );
    assert!(message.contains("line"), "YAML errors say where: {message}");
}

#[test]
fn a_value_of_the_wrong_type_is_reported_with_its_path() {
    let temp = TempDir::new().expect("tempdir");
    write_file(&settings_path(&temp), "counter: many\n");
    let error = stored(&settings_path(&temp))
        .parse::<Counter>()
        .expect_err("not a number");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(
        error
            .to_string()
            .starts_with("settings.yaml does not parse: counter: invalid type"),
        "{error}"
    );
}

#[test]
fn an_empty_or_comment_only_file_holds_no_settings() {
    let temp = TempDir::new().expect("tempdir");
    for text in ["", "# nothing here yet\n", "---\n", "~\n"] {
        write_file(&settings_path(&temp), text);
        let stored = stored(&settings_path(&temp));
        assert_eq!(
            stored.parse::<Counter>().expect("empty"),
            Counter::default()
        );
        assert!(stored.document.is_empty());
    }
}

#[test]
fn a_file_that_is_not_a_mapping_does_not_parse() {
    let temp = TempDir::new().expect("tempdir");
    for text in ["- counter\n", "counter\n", "5\n"] {
        write_file(&settings_path(&temp), text);
        let error = read(&settings_path(&temp)).expect_err("not a mapping");
        assert_eq!(
            error.to_string(),
            "settings.yaml does not parse: expected a mapping of settings"
        );
    }
}

/// No setting uses a tag, and YAML reads `!*.tmp` as a tag on an empty value:
/// refusing it is how the mask is not lost without a word.
#[test]
fn a_yaml_tag_anywhere_is_refused_with_its_path() {
    let temp = TempDir::new().expect("tempdir");
    let cases = [
        ("options:\n  Masks: !*.tmp\n", "options.Masks: !*.tmp"),
        (
            "command_line: {prompt_format: !x $p}\n",
            "command_line.prompt_format: !x",
        ),
        ("lsp:\n  go:\n    args: [run, !x a]\n", "lsp.go.args[1]: !x"),
        ("top: !x\n", "top: !x"),
    ];
    for (text, place) in cases {
        write_file(&settings_path(&temp), text);
        let error = read(&settings_path(&temp)).expect_err("a tag");
        assert_eq!(
            error.to_string(),
            format!(
                "settings.yaml does not parse: {place} is a YAML tag, which no setting \
                 uses; put a value that begins with ! in single quotes"
            )
        );
    }
    write_file(&settings_path(&temp), "masks: ['!*.tmp', [a, {b: 1}]]\n");
    assert!(read(&settings_path(&temp)).is_ok(), "a quoted ! is text");
}

#[test]
fn a_directory_or_an_oversized_file_is_refused() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    fs::create_dir_all(&path).expect("a directory where the file goes");
    let error = read(&path).expect_err("a directory is not a settings file");
    assert_eq!(error.to_string(), "settings.yaml is not a regular file");

    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    write_file(&path, "");
    File::options()
        .write(true)
        .open(&path)
        .and_then(|file| file.set_len(MAX_FILE_BYTES + 1))
        .expect("grow the file");
    let error = read(&path).expect_err("too large");
    assert_eq!(error.to_string(), "settings.yaml exceeds 1 MiB");
}

/// A save blocked by the legacy file names it, or the user never learns that
/// `settings.json` is what stands in the way.
#[test]
fn a_legacy_file_that_cannot_be_read_is_named_and_not_replaced() {
    let temp = TempDir::new().expect("tempdir");
    fs::create_dir_all(legacy_path(&temp)).expect("a directory where the file goes");
    let error = read(&settings_path(&temp)).expect_err("not a file");
    assert_eq!(error.to_string(), "settings.json is not a regular file");
    let error = save(&settings_path(&temp), 1).expect_err("refused");
    assert_eq!(
        error.to_string(),
        "Not saved: settings.json is not a regular file"
    );
    assert!(!settings_path(&temp).exists());
}

#[test]
fn an_io_error_names_the_settings_file() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    write_file(&path, "");
    fs::write(&path, b"counter: \xff\n").expect("not UTF-8");
    let error = read(&path).expect_err("not UTF-8");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().starts_with("settings.yaml: "), "{error}");
    let error = save(&path, 1).expect_err("refused");
    assert!(
        error.to_string().starts_with("Not saved: settings.yaml: "),
        "{error}"
    );
}

/// Windows reports a path through a file as missing; Unix says why.
#[cfg(unix)]
#[test]
fn a_path_through_a_file_names_the_settings_file() {
    let temp = TempDir::new().expect("tempdir");
    let blocked = temp.path().join("file");
    fs::write(&blocked, "").expect("a file where a directory goes");
    let error = read(&blocked.join(FILE_NAME)).expect_err("not a directory");
    assert!(error.to_string().starts_with("settings.yaml: "), "{error}");
}

#[test]
fn a_file_of_exactly_1_mib_is_read() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    let comment = format!("#{}\n", "x".repeat(1_048_574));
    assert_eq!(comment.len(), 1_048_576);
    write_file(&path, &comment);
    assert!(read(&path).expect("at the limit").is_some());
}

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

#[test]
fn a_small_file_is_read_as_text() {
    let temp = TempDir::new().expect("tempdir");
    let path = temp.path().join("dictionary.json");
    fs::write(&path, "{}").expect("fixture");
    assert_eq!(read_small_file(&path, "Dictionary").expect("read"), "{}");
    let missing = read_small_file(&temp.path().join("missing"), "Dictionary").expect_err("missing");
    assert_eq!(missing.kind(), io::ErrorKind::NotFound);
    assert!(missing.to_string().starts_with("Dictionary: "), "{missing}");
}
