//! Purpose: What the dialog opens on, and where availability comes from.

use super::*;

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
