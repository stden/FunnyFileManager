//! Purpose: A config FFM cannot read or parse is named, with the reason, in
//! the registration report and on the `:mcp` screen; the dialog only dims it.

use super::*;

/// Codex's config broken and a folder where Kiro's should be, on the fake
/// machine; the paths.
fn broken_configs(machine: &FakeMachine) -> (std::path::PathBuf, std::path::PathBuf) {
    let codex = machine.config(ClientId::Codex, Scope::AllProjects);
    fs::create_dir_all(codex.parent().unwrap()).unwrap();
    fs::write(&codex, "[mcp_servers\n").unwrap();
    let kiro = machine.config(ClientId::Kiro, Scope::AllProjects);
    fs::create_dir_all(&kiro).unwrap();
    (codex, kiro)
}

#[test]
fn an_unparsable_or_unreadable_config_is_named_in_the_report() {
    let (machine, mut app) = machine_app();
    let (codex, kiro) = broken_configs(&machine);
    open(&mut app);
    register(&mut app);
    let Overlay::Viewer(viewer) = &app.overlay else {
        panic!("the report opens");
    };
    let report = viewer.lines.join("\n");
    let codex_line = format!("  Codex        {} does not parse (", codex.display());
    assert!(report.contains(&codex_line), "{report}");
    assert!(
        report.contains("line 1"),
        "the parser's position:\n{report}"
    );
    let kiro_line = format!(
        "  Kiro         cannot read {}: not a regular file",
        kiro.display()
    );
    assert!(report.contains(&kiro_line), "{report}");
}

#[test]
fn a_report_with_every_config_usable_has_no_problem_section() {
    let (_machine, mut app) = machine_app();
    open(&mut app);
    register(&mut app);
    let Overlay::Viewer(viewer) = &app.overlay else {
        panic!("the report opens");
    };
    assert!(!viewer.lines.contains(&CONFIG_PROBLEMS.to_owned()));
}

#[test]
fn the_mcp_screen_names_every_config_it_cannot_use() {
    let (machine, mut app) = machine_app();
    let (codex, kiro) = broken_configs(&machine);
    crate::ops::viewer::open_mcp_status(&mut app).unwrap();
    let screen = app.get_viewer().expect("viewer").lines.join("\n");
    assert!(screen.contains(CONFIG_PROBLEMS), "{screen}");
    assert!(screen.contains(&codex.display().to_string()), "{screen}");
    assert!(
        screen.contains(&format!("cannot read {}", kiro.display())),
        "{screen}"
    );
}
