//! Purpose: unit tests for the problems the dialog reports, and which of
//! them block F2.

use super::*;

#[test]
fn a_duplicate_blocks_only_among_enabled_servers_and_only_when_new() {
    let mut editor = editor(&json!({}));
    assert!(editor
        .problems()
        .iter()
        .all(|(_, impact)| *impact == Impact::Warning));
    editor.draft.0.get_mut("python").expect("python").extensions = vec!["ts".to_owned()];
    let problems = editor.problems();
    let duplicate = Problem::DuplicateExtension {
        extension: "ts".to_owned(),
        first: language("python"),
        second: language("typescript"),
    };
    assert!(problems.contains(&(duplicate.clone(), Impact::Blocking)));
    assert_eq!(
        duplicate.to_string(),
        ".ts is claimed by python and typescript (both enabled)"
    );
    editor.draft.0.get_mut("python").expect("python").enabled = Enablement::Disabled;
    assert!(!editor
        .problems()
        .iter()
        .any(|(problem, _)| problem == &duplicate));
}

/// A conflict already in the file is a warning, even when the save changes
/// one of the servers involved.
#[test]
fn a_conflict_already_in_the_file_is_a_warning() {
    let mut editor = editor(&json!({"deno": {"command": "deno", "extensions": ["TS"]}}));
    editor.draft.0.get_mut("deno").expect("deno").args = vec!["lsp".to_owned()];
    let duplicate = editor
        .problems()
        .into_iter()
        .find(|(problem, _)| matches!(problem, Problem::DuplicateExtension { .. }))
        .expect("deno and typescript both claim ts");
    assert_eq!(duplicate.1, Impact::Warning);
    assert!(
        duplicate.0.involves(&language("deno")) && duplicate.0.involves(&language("typescript"))
    );
    assert_eq!(duplicate.0.language(), &language("deno"));
}

#[test]
fn loaded_values_that_cannot_work_are_warnings() {
    let editor = editor(&json!({
        "bad": {"command": " ", "extensions": [".rs", "d.ts"], "args": ["a\u{7}"]},
        "none": {"command": "x", "extensions": []}
    }));
    let problems: Vec<String> = editor
        .problems()
        .into_iter()
        .filter(|(problem, impact)| {
            *impact == Impact::Warning
                && (problem.involves(&language("bad")) || problem.involves(&language("none")))
        })
        .map(|(problem, _)| problem.to_string())
        .collect();
    assert_eq!(
        problems,
        [
            "lsp.bad.args holds a control character; fix settings.yaml by hand, or press Del \
             to reset a catalog server.",
            "no command — this server never starts.",
            ".rs is stored with its dot and never matches; OK in the form stores rs",
            ".d.ts can never match: FFM compares the text after the last dot.",
            "no extensions — this server never starts.",
        ]
    );
}

/// Each problem row says only what is true of OK in the form: it fixes a
/// stored `.zz`, not a `..zz`, and a stored comma is reported.
#[test]
fn an_invalid_stored_extension_is_described_truthfully() {
    let editor = editor(&json!({
        "dotted": {"command": "d", "extensions": [".zz"]},
        "doubled": {"command": "d", "extensions": ["..zz"]},
        "comma": {"command": "c", "extensions": ["qq", "a,b"]}
    }));
    let said = |key: &str| -> Vec<String> {
        editor
            .problems()
            .into_iter()
            .filter(|(problem, _)| problem.involves(&LanguageId::from_key(key)))
            .map(|(problem, _)| problem.to_string())
            .collect()
    };
    assert_eq!(
        said("dotted"),
        [".zz is stored with its dot and never matches; OK in the form stores zz"]
    );
    assert_eq!(
        said("doubled"),
        ["..zz can never match: FFM compares the text after the last dot."]
    );
    assert_eq!(
        said("comma"),
        ["Extensions cannot contain spaces, commas, / or \\."]
    );
}

/// The round trip the problem row promises: OK in the form, touching
/// nothing, stores a dotted extension without its dot.
#[test]
fn ok_in_the_form_stores_a_dotted_extension_without_its_dot() {
    let mut editor = editor(&json!({"mine": {"command": "m", "extensions": [".zz", "yy"]}}));
    editor
        .open_form(&FormTarget::Existing(language("mine")))
        .expect("the form opens");
    assert_eq!(editor.accept_form(), Some(language("mine")));
    assert_eq!(editor.draft().0["mine"].extensions, ["zz", "yy"]);
}
