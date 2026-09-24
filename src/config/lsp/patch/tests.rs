//! Purpose: Pin the smallest-change rule: only changed owned fields are
//! written, a catalog value is unset rather than pinned, every other key
//! keeps its value and its place, and a save is refused when `lsp` changed
//! on disk.

use std::fs;

use tempfile::TempDir;

use super::*;
use crate::config::{Enablement, InstallRecipe};

/// `text` as YAML: a mapping, or a scalar.
fn yaml<T: serde::de::DeserializeOwned>(text: &str) -> T {
    serde_norway::from_str(text).expect("valid YAML")
}

/// The table loaded from `lsp` as the file holds it, and that raw mapping.
fn loaded(lsp: &str) -> (LspSettings, Mapping) {
    (yaml(lsp), yaml(lsp))
}

/// `lsp` after the draft that `edit` makes of it is saved.
fn saved(lsp: &str, edit: impl FnOnce(&mut LspSettings)) -> Mapping {
    let (baseline, mut raw) = loaded(lsp);
    let mut draft = baseline.clone();
    edit(&mut draft);
    apply(&diff(&baseline, &draft), &mut raw).expect("applies");
    raw
}

fn server<'a>(settings: &'a mut LspSettings, language: &str) -> &'a mut ServerSettings {
    settings.0.get_mut(language).expect("a server")
}

fn python() -> LanguageId {
    LanguageId::from_key("python")
}

/// The top-level keys of `mapping`, in order.
fn keys(mapping: &Mapping) -> Vec<&str> {
    mapping.keys().filter_map(Value::as_str).collect()
}

#[test]
fn disabling_a_catalog_server_sets_the_field_and_enabling_it_again_unsets_it() {
    let (baseline, _) = loaded("{}");
    let mut draft = baseline.clone();
    server(&mut draft, "python").enabled = Enablement::Disabled;
    assert_eq!(
        diff(&baseline, &draft),
        [Change::Set {
            language: python(),
            field: OwnedField::Enabled,
            value: Value::Bool(false),
        }]
    );
    let disabled = saved("{}", |draft| {
        server(draft, "python").enabled = Enablement::Disabled;
    });
    assert_eq!(disabled, yaml::<Mapping>("python: {enabled: false}"));

    let text = serde_norway::to_string(&disabled).expect("serialize");
    let enabled = saved(&text, |draft| {
        server(draft, "python").enabled = Enablement::Enabled;
    });
    assert_eq!(enabled, Mapping::new(), "the entry goes once it is empty");
}

#[test]
fn an_untouched_table_has_no_changes() {
    let (baseline, _) = loaded("{python: {command: custom}, mine: {command: x}}");
    assert_eq!(diff(&baseline, &baseline.clone()), []);
    assert_eq!(Edit::new(&baseline, &baseline), None);
}

/// A server the catalog does not know is written whole: an omitted field
/// would fall back to the defaults of an unknown language, rust-analyzer's.
#[test]
fn an_added_server_is_written_in_full() {
    let written = saved("{}", |draft| {
        draft.0.insert(
            "fixture".to_owned(),
            ServerSettings {
                enabled: Enablement::Enabled,
                command: "/srv/x/server".to_owned(),
                args: vec!["--stdio".to_owned()],
                extensions: vec!["fx".to_owned()],
                root_markers: vec![".git".to_owned()],
                install: None,
            },
        );
    });
    assert_eq!(
        written,
        yaml::<Mapping>(
            "fixture: {enabled: true, command: /srv/x/server, args: [--stdio], \
             extensions: [fx], root_markers: [.git], install: null}"
        )
    );
}

/// Reset puts the four owned fields back to the catalog; what the dialog
/// does not own — markers, a disabled recipe, keys of a newer FFM — stays.
#[test]
fn a_reset_keeps_the_fields_the_dialog_does_not_own() {
    let file = "python: {command: custom, root_markers: [setup.cfg], install: null, future: 1}";
    let written = saved(file, |draft| {
        let catalog = &LspSettings::catalog().0["python"];
        let python = server(draft, "python");
        python.command.clone_from(&catalog.command);
        python.enabled = catalog.enabled;
    });
    assert_eq!(
        written,
        yaml::<Mapping>("python: {root_markers: [setup.cfg], install: null, future: 1}")
    );
}

#[test]
fn an_added_server_that_is_removed_is_dropped_and_the_rest_keep_their_order() {
    let file = "{zeta: {command: z}, mine: {command: m}, alpha: {command: a}}";
    let written = saved(file, |draft| {
        draft.0.remove("mine");
    });
    assert_eq!(keys(&written), ["zeta", "alpha"]);
}

/// An added server has no catalog value to fall back to: a changed field is
/// always set, never unset.
#[test]
fn a_changed_added_server_is_set_field_by_field() {
    let (baseline, _) = loaded("mine: {command: old, extensions: [m], root_markers: [.git]}");
    let mut draft = baseline.clone();
    server(&mut draft, "mine").command = "rust-analyzer".to_owned();
    assert_eq!(
        diff(&baseline, &draft),
        [Change::Set {
            language: LanguageId::from_key("mine"),
            field: OwnedField::Command,
            value: Value::from("rust-analyzer"),
        }]
    );
}

#[test]
fn an_unset_keeps_the_order_of_the_other_fields() {
    let file = "python: {install: null, command: custom, args: [-v], enabled: false}";
    let written = saved(file, |draft| {
        let catalog = &LspSettings::catalog().0["python"];
        server(draft, "python").command.clone_from(&catalog.command);
    });
    let python = written["python"].as_mapping().expect("mapping");
    assert_eq!(keys(python), ["install", "args", "enabled"]);
}

#[test]
fn a_set_on_a_catalog_server_the_file_lacks_creates_its_mapping() {
    let written = saved("go: {command: gopls}", |draft| {
        server(draft, "python").args = vec!["-v".to_owned()];
    });
    assert_eq!(
        written,
        yaml::<Mapping>("{go: {command: gopls}, python: {args: [-v]}}")
    );
}

#[test]
fn an_entry_that_is_not_a_mapping_is_reported() {
    let mut raw: Mapping = yaml("python: 5");
    let set = [Change::Set {
        language: python(),
        field: OwnedField::Enabled,
        value: Value::Bool(false),
    }];
    let unset = [Change::Unset {
        language: python(),
        field: OwnedField::Enabled,
    }];
    for changes in [&set[..], &unset[..]] {
        let error = apply(changes, &mut raw).expect_err("not a mapping");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            error.to_string(),
            "lsp.python in settings.yaml is not a mapping"
        );
    }
    assert_eq!(raw["python"], Value::from(5), "left as it was");
}

/// Back at the catalog's value for a server the file has no entry for:
/// there is nothing to remove.
#[test]
fn an_unset_of_a_server_the_file_lacks_changes_nothing() {
    let mut raw = Mapping::new();
    let unset = [Change::Unset {
        language: python(),
        field: OwnedField::Command,
    }];
    apply(&unset, &mut raw).expect("nothing to do");
    assert_eq!(raw, Mapping::new());
}

/// A file whose `lsp` is neither a mapping nor empty does not parse, so a
/// save refuses before it gets here; were it reached, it still writes nothing.
#[test]
fn an_lsp_that_is_not_a_mapping_is_reported() {
    let mut document: Mapping = yaml("lsp: 5");
    let error = lsp_mapping(&mut document).expect_err("not a mapping");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(error.to_string(), "lsp in settings.yaml is not a mapping");
    let mut document = Mapping::new();
    assert_eq!(
        lsp_mapping(&mut document).map(|lsp| lsp.clone()).ok(),
        Some(Mapping::new()),
        "a file without `lsp` gets an empty one"
    );
}

/// The four keys cannot drift from `ServerSettings`' serde names.
#[test]
fn every_owned_key_is_the_serde_name_of_its_field() {
    let server = ServerSettings {
        install: Some(InstallRecipe::Python {
            package: "p".to_owned(),
            module: "m".to_owned(),
        }),
        ..ServerSettings::default()
    };
    let value = serde_norway::to_value(&server).expect("serialize");
    for field in OwnedField::ALL {
        assert_eq!(
            value[field.key()],
            field.value_of(&server),
            "{}",
            field.key()
        );
    }
}

/// An added server removed and added again in one draft is a new server: it
/// is written whole, so the old entry's other keys go with it, even when the
/// owned fields come back unchanged.
#[test]
fn an_added_server_removed_and_added_again_is_written_whole() {
    let lsp = "zig: {command: zls, extensions: [zig], root_markers: [build.zig], custom: 1}";
    for command in ["zls2", "zls"] {
        let replaced = saved(lsp, |draft| {
            draft.0.remove("zig");
            draft.0.insert(
                "zig".to_owned(),
                ServerSettings {
                    command: command.to_owned(),
                    extensions: vec!["zig".to_owned()],
                    root_markers: vec![".git".to_owned()],
                    install: None,
                    ..ServerSettings::default()
                },
            );
        });
        assert_eq!(replaced["zig"]["command"], Value::from(command));
        assert_eq!(replaced["zig"]["root_markers"], yaml::<Value>("[.git]"));
        assert_eq!(replaced["zig"]["install"], Value::Null);
        assert!(replaced["zig"].get("custom").is_none(), "{replaced:?}");
    }
}

/// The four owned fields come from `from`; the recipe and the root markers
/// stay the server's own.
#[test]
fn with_owned_fields_copies_the_owned_four_and_keeps_the_rest() {
    let python = LspSettings::catalog().0["python"].clone();
    let from = ServerSettings {
        enabled: Enablement::Disabled,
        command: "p".to_owned(),
        args: vec!["-v".to_owned()],
        extensions: vec!["pp".to_owned()],
        root_markers: vec!["elsewhere".to_owned()],
        install: None,
    };
    let merged = with_owned_fields(python.clone(), &from);
    for field in OwnedField::ALL {
        assert_eq!(field.value_of(&merged), field.value_of(&from), "{field:?}");
    }
    assert_eq!(merged.root_markers, python.root_markers);
    assert_eq!(merged.install, python.install);
}

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
