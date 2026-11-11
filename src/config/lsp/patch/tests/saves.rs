//! Purpose: Pin the save: an empty `lsp` becomes a mapping, a change to `lsp`
//! on disk refuses it, other keys changed on disk do not, and every string
//! reads back the same.

use super::*;

/// A settings file in `temp` holding `text`, and the servers it loads as.
fn settings_file(temp: &TempDir, text: &str) -> (std::path::PathBuf, LspSettings) {
    let path = temp.path().join(document::FILE_NAME);
    fs::write(&path, text).expect("fixture");
    let servers = Settings::load_from(&path).expect("loads").lsp;
    (path, servers)
}

/// Disable python in `servers`: the edit every save test makes.
fn python_disabled(servers: &LspSettings) -> Edit {
    let mut draft = servers.clone();
    server(&mut draft, "python").enabled = Enablement::Disabled;
    Edit::new(servers, &draft).expect("a change")
}

/// The file's text as YAML.
fn on_disk(path: &Path) -> Mapping {
    yaml(&fs::read_to_string(path).expect("settings file"))
}

/// `lsp:` and `lsp: ~` load as the catalog; a save replaces the null with the
/// mapping of the change, and leaves every other key as it was.
#[test]
fn a_save_turns_an_empty_lsp_into_a_mapping() {
    for text in [
        "lsp:\nsystem: {history_size: 5}\n",
        "lsp: ~\nsystem: {history_size: 5}\n",
    ] {
        let temp = TempDir::new().expect("temp");
        let (path, servers) = settings_file(&temp, text);
        assert_eq!(&servers, LspSettings::catalog());
        let written = python_disabled(&servers).save(&path).expect("saved");
        assert_eq!(written.0["python"].enabled, Enablement::Disabled);
        assert_eq!(
            on_disk(&path),
            yaml::<Mapping>("{lsp: {python: {enabled: false}}, system: {history_size: 5}}")
        );
    }
}

/// Another program changed `lsp` since the dialog read it: the save would
/// write over that change, so it is refused and the file stays as it is.
#[test]
fn a_save_is_refused_when_lsp_changed_on_disk() {
    let temp = TempDir::new().expect("temp");
    let (path, servers) = settings_file(&temp, "lsp: {go: {command: gopls}}\n");
    let edit = python_disabled(&servers);
    let changed = "lsp: {go: {command: gopls2}}\n";
    fs::write(&path, changed).expect("the other program's edit");
    let error = edit.save(&path).expect_err("refused");
    assert_eq!(
        error.to_string(),
        "Not saved: lsp in settings.yaml changed outside this dialog; close without saving \
         and open it again to see the change"
    );
    assert_eq!(fs::read_to_string(&path).expect("file"), changed);
}

/// Edits outside `lsp`, and a respelling of it that loads as the same
/// servers, do not block a save; the other program's keys survive it.
#[test]
fn a_save_goes_ahead_when_only_other_keys_changed_on_disk() {
    let temp = TempDir::new().expect("temp");
    let (path, servers) = settings_file(&temp, "lsp: {go: {command: gopls}}\n");
    let edit = python_disabled(&servers);
    fs::write(
        &path,
        "system: {history_size: 9}\nlsp:\n  go:\n    command: 'gopls'\n",
    )
    .expect("the other program's edit");
    edit.save(&path).expect("saved");
    assert_eq!(
        on_disk(&path),
        yaml::<Mapping>(
            "{system: {history_size: 9}, lsp: {go: {command: gopls}, python: {enabled: false}}}"
        )
    );
}

/// Every string the dialog can write reads back as that string, however
/// YAML would read it unquoted.
#[test]
fn every_string_the_dialog_writes_reads_back_the_same() {
    let mut strings = crate::config::tests::persistence::tricky();
    strings.extend(
        [
            "true", "-", "*x", "'q'", "\"dq\"", "{x}", "--stdio", "1e3", "%x",
        ]
        .map(str::to_owned),
    );
    let temp = TempDir::new().expect("temp");
    let path = temp.path().join(document::FILE_NAME);
    let servers = LspSettings::default();
    let mut draft = servers.clone();
    for (index, text) in strings.iter().enumerate() {
        draft.0.insert(
            format!("added{index}"),
            ServerSettings {
                command: text.clone(),
                args: vec![text.clone()],
                extensions: vec![text.clone()],
                root_markers: vec![text.clone()],
                ..ServerSettings::default()
            },
        );
    }
    let written = Edit::new(&servers, &draft)
        .expect("a change")
        .save(&path)
        .expect("saved");
    assert_eq!(written, draft);
    assert_eq!(Settings::load_from(&path).expect("loads").lsp, draft);
}
