//! Purpose: What F2 writes, and every save that is refused or fails.

use super::*;

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
