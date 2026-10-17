//! Purpose: Pin the diff and what it writes: a changed owned field is set, one
//! back at the catalog's value is unset, and an added server is written whole.

use super::*;

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
