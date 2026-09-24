//! Purpose: Pin how the dialog opens on the settings file, what F2 writes, and
//! that every refused or failed save leaves the file and the draft alone.

use std::fs;

use super::*;
use crate::config::Enablement;
use crate::lsp::NotInstalled;
use crate::models::lsp_editor::Mode;
use crate::models::settings::Settings;

use crate::test_support::lsp_machine::{editor, editor_mut, LspMachine as Machine};

/// `text` as YAML, to compare with what the file holds.
fn yaml(text: &str) -> serde_norway::Value {
    serde_norway::from_str(text).expect("YAML")
}

fn read_only_reason(app: &App) -> String {
    match editor(app).access() {
        Access::ReadOnly(reason) => reason.to_string(),
        Access::Writable { path } => panic!("writable at {}", path.display()),
    }
}

#[test]
fn a_missing_file_opens_the_catalog_writable() {
    let machine = Machine::new();
    let app = machine.opened();
    let editor = editor(&app);
    assert_eq!(
        editor.access(),
        &Access::Writable {
            path: machine.settings()
        }
    );
    assert_eq!(editor.draft(), LspSettings::catalog());
    assert_eq!(editor.mode(), &Mode::Browse);
}

#[test]
fn a_file_that_is_not_yaml_opens_read_only_with_its_position() {
    let machine = Machine::new();
    machine.write_settings("{broken");
    let mut app = machine.app();
    app.config.lsp.0.retain(|language, _| language == "rust");
    open(&mut app);
    let reason = read_only_reason(&app);
    assert!(
        reason.starts_with("settings.yaml does not parse: "),
        "{reason}"
    );
    assert!(reason.contains("line 2 column 1"), "{reason}");
    assert_eq!(
        editor(&app).draft(),
        &app.config.lsp,
        "what FFM would start"
    );
}

#[test]
fn an_lsp_entry_that_does_not_parse_opens_read_only_naming_it() {
    let machine = Machine::new();
    machine.write_settings("lsp:\n  python:\n    args: x\n");
    let app = machine.opened();
    let reason = read_only_reason(&app);
    assert!(
        reason.starts_with("settings.yaml does not parse: lsp: python: "),
        "{reason}"
    );
}

#[test]
fn a_directory_in_place_of_the_file_opens_read_only() {
    let machine = Machine::new();
    fs::create_dir(machine.settings()).expect("dir");
    let app = machine.opened();
    assert_eq!(
        read_only_reason(&app),
        "settings.yaml is not a regular file"
    );
}

#[test]
fn a_session_without_a_file_shows_what_ffm_would_start() {
    let machine = Machine::new();
    let mut app = machine.app();
    app.settings.persistence = crate::history::Persistence::SessionOnly;
    app.config.lsp.0.retain(|language, _| language == "python");
    open(&mut app);
    assert_eq!(read_only_reason(&app), "no settings file for this session");
    assert_eq!(editor(&app).draft(), &app.config.lsp);
}

/// Startup remembers the file even when the Options pages fail to load, so
/// the dialog knows the file and judges it for itself.
#[test]
fn a_failed_options_load_still_tells_the_dialog_where_the_file_is() {
    let machine = Machine::new();
    machine.write_settings("{broken");
    let mut app = machine.app();
    app.settings = Settings::default();
    crate::ops::menu::settings::restore(&mut app, machine.settings());
    open(&mut app);
    assert!(read_only_reason(&app).contains("does not parse"));

    machine.write_settings("options:\n  Editor: '20'\nlsp:\n  python:\n    enabled: false\n");
    let mut app = machine.app();
    app.settings = Settings::default();
    assert!(crate::ops::menu::settings::load(&mut app, &machine.settings()).is_err());
    crate::ops::menu::settings::restore(&mut app, machine.settings());
    open(&mut app);
    assert!(matches!(editor(&app).access(), Access::Writable { .. }));
    assert_eq!(
        editor(&app).draft().0["python"].enabled,
        Enablement::Disabled
    );
}

#[test]
fn availability_comes_from_the_apps_probe_only() {
    let machine = Machine::new();
    let pylsp = machine.bin().join(crate::lsp::program_file("pylsp"));
    crate::test_support::write_executable(&pylsp, b"pylsp").expect("program");
    let app = machine.opened();
    let found = editor(&app)
        .availability(&LanguageId::from_key("python"))
        .expect("checked");
    assert!(matches!(found.resolved(), Some(resolved) if resolved.executable == pylsp));

    let mut bare = machine.app();
    bare.live.lsp_probe = Probe::default();
    open(&mut bare);
    assert_eq!(
        editor(&bare).availability(&LanguageId::from_key("python")),
        Some(&crate::lsp::Availability::NotInstalled(
            NotInstalled::NoCache
        ))
    );
}

#[test]
fn a_failing_write_keeps_the_draft_the_baseline_and_the_file() {
    let machine = Machine::new();
    machine.write_settings("options:\n  System: '5000'\n");
    let mut app = machine.opened();
    editor_mut(&mut app).select(&LanguageId::from_key("python"));
    toggle(&mut app);
    let outcome = save_with(&mut app, |_, _| {
        Err(io::Error::new(
            io::ErrorKind::StorageFull,
            "Not saved: settings.yaml: no space left on device",
        ))
    });
    assert_eq!(outcome, SaveOutcome::Refused);
    let editor = editor(&app);
    assert!(editor.dirty(), "the draft survives for another F2");
    assert_eq!(editor.baseline(), LspSettings::catalog());
    assert!(matches!(
        editor.notice(),
        Some(Notice::Error(message)) if message == "Not saved: settings.yaml: no space left on device"
    ));
    assert_eq!(machine.parsed(), yaml("options:\n  System: '5000'\n"));
    assert_eq!(
        app.config.lsp,
        LspSettings::default(),
        "the editor keeps what it had"
    );
}

#[test]
fn a_save_writes_the_change_and_keeps_the_rest() {
    let machine = Machine::new();
    machine.write_settings("options:\n  System: '5000'\nunknown:\n  keep: 1\n");
    let mut app = machine.opened();
    editor_mut(&mut app).select(&LanguageId::from_key("python"));
    toggle(&mut app);
    assert_eq!(save(&mut app), SaveOutcome::Saved);
    assert_eq!(
        machine.parsed(),
        yaml(
            "options:\n  System: '5000'\nunknown:\n  keep: 1\nlsp:\n  python:\n    enabled: false\n"
        )
    );
    let backup = machine.settings().with_extension("yaml.bak");
    assert!(!backup.exists(), "a save keeps no backup");
    assert_eq!(
        app.config.lsp.0["python"].enabled,
        Enablement::Disabled,
        "the next editor start uses the file"
    );
    let editor = editor(&app);
    assert!(!editor.dirty());
    assert_eq!(editor.baseline(), &app.config.lsp);
    assert!(matches!(
        editor.notice(),
        Some(Notice::Info(message)) if message.starts_with("Saved to ")
    ));
}

/// Another program edited `lsp` while the dialog was open: nothing is
/// written, and the draft is kept.
#[test]
fn an_lsp_changed_on_disk_refuses_the_save() {
    let machine = Machine::new();
    let mut app = machine.opened();
    machine.write_settings("lsp:\n  rust:\n    enabled: false\n");
    toggle(&mut app);
    assert_eq!(save(&mut app), SaveOutcome::Refused);
    assert!(matches!(
        editor(&app).notice(),
        Some(Notice::Error(message))
            if message.starts_with("Not saved: lsp in settings.yaml changed outside this dialog")
    ));
    assert_eq!(
        fs::read_to_string(machine.settings()).expect("settings"),
        "lsp:\n  rust:\n    enabled: false\n"
    );
    assert!(editor(&app).dirty());
}

/// An edit outside `lsp` does not block the dialog's save.
#[test]
fn an_edit_outside_lsp_does_not_block_the_save() {
    let machine = Machine::new();
    machine.write_settings("options:\n  Editor: '8'\n");
    let mut app = machine.opened();
    machine.write_settings("options:\n  Editor: '6'\n");
    toggle(&mut app);
    assert_eq!(save(&mut app), SaveOutcome::Saved);
    assert_eq!(machine.parsed()["options"]["Editor"], yaml("'6'"));
}

#[test]
fn a_blocking_problem_refuses_the_save_and_selects_the_server() {
    let machine = Machine::new();
    let mut app = machine.opened();
    open_form(
        &mut app,
        &FormTarget::Existing(LanguageId::from_key("python")),
    );
    let form = editor_mut(&mut app).form_mut().expect("form");
    form.focus_on(crate::models::lsp_editor::FormField::Extensions);
    form.focused_line_mut().expect("line").set_text("py ts");
    accept_form(&mut app);
    editor_mut(&mut app).move_selection(crate::models::lsp_editor::Step::First);
    assert_eq!(save(&mut app), SaveOutcome::Refused);
    let editor = editor(&app);
    assert_eq!(
        editor.notice(),
        Some(&Notice::Error(
            "Not saved: .ts is claimed by python and typescript (both enabled).".to_owned()
        ))
    );
    assert_eq!(editor.selected().map(LanguageId::as_str), Some("python"));
    assert!(!machine.settings().exists(), "nothing is written");
}

#[test]
fn a_clean_draft_has_nothing_to_save_and_a_read_only_one_says_why() {
    let machine = Machine::new();
    let mut app = machine.opened();
    assert_eq!(save(&mut app), SaveOutcome::Unchanged);
    assert_eq!(
        editor(&app).notice(),
        Some(&Notice::Info("Nothing to save.".to_owned()))
    );
    assert!(!machine.settings().exists());

    let mut app = machine.app();
    app.settings.persistence = crate::history::Persistence::SessionOnly;
    open(&mut app);
    toggle(&mut app);
    assert!(!editor(&app).dirty(), "a read-only table does not change");
    assert_eq!(save(&mut app), SaveOutcome::Refused);
    assert_eq!(
        editor(&app).notice(),
        Some(&Notice::Error(
            "Read only: no settings file for this session.".to_owned()
        ))
    );
}

#[test]
fn the_lsp_command_and_the_commands_menu_row_open_the_dialog() {
    use crate::menu::{Heading, Menu};

    let machine = Machine::new();
    let mut app = machine.app();
    crate::ops::execute_command(&mut app, "lsp").expect("the lsp command");
    assert!(matches!(app.overlay, Overlay::LspEditor(_)));

    let mut app = machine.app();
    let entry = Menu::open(Heading::Commands)
        .entries()
        .iter()
        .find(|entry| crate::menu::label_of(entry).contains("Language servers"))
        .copied()
        .expect("LSP menu entry");
    crate::ops::menu::activate(&mut app, entry);
    assert!(matches!(app.overlay, Overlay::LspEditor(_)));
}

#[path = "tests_edit.rs"]
mod edit;
