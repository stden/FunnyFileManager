//! Unit tests: which file a load reads — `settings.yaml`, else the legacy
//! JSON — and what it reads from it, down to the path of a value that does
//! not parse and the YAML tags no setting uses.

use super::*;

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
        assert_eq!(stored.document.len(), 0);
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
        refused_as_tag(&temp, text, place);
    }
    write_file(&settings_path(&temp), "masks: ['!*.tmp', [a, {b: 1}]]\n");
    assert!(read(&settings_path(&temp)).is_ok(), "a quoted ! is text");
}

/// The two origin tags an earlier build wrote under `sessions` are read, and
/// they admit no other tag: `Masks: !*.tmp` beside one is refused as it is on
/// its own, and a tag in origin position that is not one of the two is refused.
#[test]
fn only_the_earlier_origin_tags_are_read() {
    let temp = TempDir::new().expect("tempdir");
    let cases = [
        (
            "sessions:\n- origin: !Things\n    machine: m\noptions:\n  Masks: !*.tmp\n",
            "options.Masks: !*.tmp",
        ),
        (
            "sessions:\n- origin: !Elsewhere 1\n",
            "sessions[0].origin: !Elsewhere",
        ),
    ];
    for (text, place) in cases {
        refused_as_tag(&temp, text, place);
    }
}

/// `text` is refused, and the error names the YAML tag at `place`.
fn refused_as_tag(temp: &TempDir, text: &str, place: &str) {
    write_file(&settings_path(temp), text);
    let error = read(&settings_path(temp)).expect_err("a tag");
    assert_eq!(
        error.to_string(),
        format!(
            "settings.yaml does not parse: {place} is a YAML tag, which no setting \
             uses; put a value that begins with ! in single quotes"
        )
    );
}
