//! Purpose: Pin the dialog state: the selection, the draft's changes, Del's
//! two meanings, what a save does to the table, and which problems block F2.

use serde_json::{json, Value};

use super::*;
use crate::config::Enablement;

/// A writable dialog over the catalog with `file`'s `lsp` object laid on it.
fn editor(file: &Value) -> LspEditor {
    let loaded: LspSettings = serde_json::from_value(file.clone()).expect("a valid lsp object");
    LspEditor::new(
        Access::Writable {
            path: "/srv/x/settings.yaml".into(),
        },
        loaded,
    )
}

fn language(key: &str) -> LanguageId {
    LanguageId::from_key(key)
}

fn selected(editor: &LspEditor) -> &str {
    editor.selected().expect("a selection").as_str()
}

#[test]
fn the_dialog_opens_on_the_first_server_and_clean() {
    let editor = editor(&json!({}));
    assert_eq!(selected(&editor), "bat");
    assert_eq!(editor.selected_index(), Some(0));
    assert!(!editor.dirty());
    assert_eq!(editor.mode(), &Mode::Browse);
    assert_eq!(editor.notice(), None);
    assert!(matches!(editor.access(), Access::Writable { .. }));
}

#[test]
fn the_selection_moves_within_the_list() {
    let mut editor = editor(&json!({}));
    let count = editor.draft().0.len();
    editor.move_selection(Step::Up);
    assert_eq!(editor.selected_index(), Some(0), "stops at the top");
    editor.move_selection(Step::Last);
    assert_eq!(editor.selected_index(), Some(count - 1));
    editor.move_selection(Step::Down);
    assert_eq!(
        editor.selected_index(),
        Some(count - 1),
        "stops at the bottom"
    );
    editor.record_drawn(Drawn::List { page: 4 });
    editor.move_selection(Step::PageUp);
    assert_eq!(editor.selected_index(), Some(count - 5));
    editor.move_selection(Step::PageDown);
    assert_eq!(editor.selected_index(), Some(count - 1));
    editor.move_selection(Step::First);
    assert_eq!(selected(&editor), "bat");
    editor.select(&language("python"));
    assert_eq!(selected(&editor), "python");
    editor.select(&language("not-there"));
    assert_eq!(
        selected(&editor),
        "python",
        "an unknown language is not selected"
    );
}

#[test]
fn a_toggled_server_is_changed_and_the_dialog_dirty() {
    let mut editor = editor(&json!({}));
    editor.select(&language("python"));
    assert_eq!(editor.toggle_selected(), Some(language("python")));
    assert!(editor.dirty());
    assert!(editor.is_changed(&language("python")));
    assert!(!editor.is_changed(&language("css")));
    assert_eq!(editor.draft().0["python"].enabled, Enablement::Disabled);
    editor.toggle_selected();
    assert!(!editor.dirty(), "back to what the file holds");
}

/// Del resets a catalog server's four owned fields and keeps the rest; on an
/// added server it removes it and selects the row now at its place.
#[test]
fn del_resets_a_catalog_server_and_removes_an_added_one() {
    let mut editor = editor(&json!({
        "python": {"command": "custom", "root_markers": ["setup.cfg"]},
        "mine": {"command": "mine-server", "extensions": ["mn"]}
    }));
    editor.select(&language("python"));
    assert_eq!(
        editor.delete_selected(),
        Some((language("python"), Origin::Catalog))
    );
    let python = &editor.draft().0["python"];
    assert_eq!(python.command, LspSettings::catalog().0["python"].command);
    assert_eq!(python.root_markers, ["setup.cfg"]);

    editor.select(&language("mine"));
    let index = editor.selected_index();
    assert_eq!(
        editor.delete_selected(),
        Some((language("mine"), Origin::Added))
    );
    assert!(!editor.draft().0.contains_key("mine"));
    assert_eq!(
        editor.selected_index(),
        index,
        "the next row takes its place"
    );
    assert_eq!(LspEditor::origin(&language("python")), Origin::Catalog);
    assert_eq!(LspEditor::origin(&language("mine")), Origin::Added);
}

/// A save brings in servers another program added or removed; the selection
/// stays on its language, and is clamped only when that language is gone.
#[test]
fn the_selection_follows_its_language_across_a_save() {
    let mut editor = editor(&json!({"zeta": {"command": "z"}}));
    editor.select(&language("python"));
    let before = editor.selected_index().expect("python");
    let with_earlier: LspSettings =
        serde_json::from_value(json!({"aaa": {"command": "a"}, "zeta": {"command": "z"}}))
            .expect("valid");
    editor.saved(with_earlier);
    assert_eq!(selected(&editor), "python");
    assert_eq!(editor.selected_index(), Some(before + 1));
    assert!(!editor.dirty());

    editor.move_selection(Step::Last);
    assert_eq!(selected(&editor), "zeta");
    let without: LspSettings = serde_json::from_value(json!({})).expect("valid");
    editor.saved(without);
    assert_eq!(selected(&editor), "yaml", "clamped to the new last row");
}

#[test]
fn availability_is_kept_per_language() {
    let mut editor = editor(&json!({}));
    let missing = Availability::NotInstalled(crate::lsp::NotInstalled::NoCache);
    editor.set_availability(language("python"), missing.clone());
    assert_eq!(editor.availability(&language("python")), Some(&missing));
    assert_eq!(editor.availability(&language("css")), None);
}

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

#[test]
fn the_view_scrolls_within_what_the_frame_drew() {
    let mut editor = editor(&json!({}));
    editor.set_mode(Mode::View(ScrollTop::TOP));
    editor.record_drawn(Drawn::View {
        lines: 50,
        page: 20,
    });
    editor.scroll_view(Step::Last);
    assert_eq!(editor.mode(), &Mode::View(ScrollTop::TOP.down(30, 30)));
    editor.scroll_view(Step::Down);
    assert_eq!(
        editor.mode(),
        &Mode::View(ScrollTop::TOP.down(30, 30)),
        "stops at the end"
    );
    editor.scroll_view(Step::PageUp);
    assert_eq!(editor.mode(), &Mode::View(ScrollTop::TOP.down(10, 30)));
    editor.scroll_view(Step::Up);
    editor.scroll_view(Step::PageDown);
    assert_eq!(editor.mode(), &Mode::View(ScrollTop::TOP.down(29, 30)));
    editor.scroll_view(Step::First);
    assert_eq!(editor.mode(), &Mode::View(ScrollTop::TOP));
}

#[test]
fn a_frame_with_fewer_lines_pulls_the_view_back_in_range() {
    let mut editor = editor(&json!({}));
    editor.set_mode(Mode::View(ScrollTop::TOP));
    editor.record_drawn(Drawn::View {
        lines: 50,
        page: 20,
    });
    editor.scroll_view(Step::Last);
    editor.record_drawn(Drawn::View {
        lines: 25,
        page: 20,
    });
    assert_eq!(editor.mode(), &Mode::View(ScrollTop::TOP.down(5, 5)));
}

#[test]
fn a_read_only_reason_reads_as_a_sentence() {
    assert_eq!(
        ReadOnlyReason::NoSettingsFile.to_string(),
        "no settings file for this session"
    );
    let reason = ReadOnlyReason::Document {
        path: "/srv/x/settings.yaml".into(),
        reason: "settings.yaml does not parse: expected a mapping of settings".to_owned(),
    };
    assert_eq!(
        reason.to_string(),
        "settings.yaml does not parse: expected a mapping of settings"
    );
}

#[path = "tests_form.rs"]
mod form_tests;

#[test]
fn the_accessors_show_the_selected_server_the_baseline_and_the_notice() {
    let mut editor = editor(&json!({}));
    editor.select(&language("python"));
    let (selected, server) = editor.selected_server().expect("python");
    assert_eq!(selected, &language("python"));
    assert_eq!(server, &editor.baseline().0["python"]);
    editor.set_notice(Notice::Info("Nothing to save".to_owned()));
    assert_eq!(
        editor.notice(),
        Some(&Notice::Info("Nothing to save".to_owned()))
    );
    *editor.mode_mut() = Mode::ConfirmExit(ExitChoice::Discard);
    assert_eq!(editor.mode(), &Mode::ConfirmExit(ExitChoice::Discard));
    assert!(editor.form_mut().is_none());
}

#[test]
fn an_empty_table_has_no_selection_to_move_or_toggle() {
    let mut editor = LspEditor::new(
        Access::ReadOnly(ReadOnlyReason::NoSettingsFile),
        LspSettings(BTreeMap::new()),
    );
    assert_eq!(editor.selected(), None);
    editor.move_selection(Step::Down);
    assert_eq!(editor.selected_index(), None);
    assert_eq!(editor.toggle_selected(), None);
    assert_eq!(editor.delete_selected(), None);
    assert_eq!(editor.selected_server(), None);
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

#[test]
fn a_cleared_notice_leaves_the_message_row_to_the_changes() {
    let mut editor = editor(&json!({}));
    editor.set_notice(Notice::Info("Checked every server again.".to_owned()));
    editor.clear_notice();
    assert_eq!(editor.notice(), None);
}

/// The one sentence a read-only dialog gives, on its message row and for
/// every change it refuses.
#[test]
fn a_read_only_dialog_says_why_in_one_sentence() {
    let writable = editor(&json!({}));
    assert_eq!(writable.read_only_notice(), None);
    let read_only = LspEditor::new(
        Access::ReadOnly(ReadOnlyReason::NoSettingsFile),
        LspSettings::default(),
    );
    assert_eq!(
        read_only.read_only_notice(),
        Some(Notice::Error(
            "Read only: no settings file for this session.".to_owned()
        ))
    );
}
