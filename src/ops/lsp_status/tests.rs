//! Purpose: Verify the lsp command opens a normal report without starting external processes.

use crate::{
    config::ServerSettings,
    models::{App, Overlay, Panel},
};

#[test]
fn lsp_command_opens_a_scrolling_inventory() {
    let root = tempfile::TempDir::new().unwrap();
    let executable = root.path().join("server");
    crate::test_support::write_executable(&executable, b"fixture").unwrap();
    let panel = Panel::new(root.path().to_path_buf()).unwrap();
    let mut app = App::with_panels(panel.clone(), panel);
    app.config.lsp.0.clear();
    app.config.lsp.0.insert(
        "test".into(),
        ServerSettings {
            command: executable.to_string_lossy().into_owned(),
            ..ServerSettings::default()
        },
    );
    crate::ops::execute_command(&mut app, "lsp").unwrap();
    let Overlay::Viewer(viewer) = app.overlay else {
        panic!("expected inventory")
    };
    assert!(viewer
        .lines
        .iter()
        .any(|line| line.contains("found configured executable")));
}

#[test]
fn commands_menu_opens_the_same_inventory() {
    use crate::menu::{Heading, Menu};
    let root = tempfile::TempDir::new().unwrap();
    let panel = Panel::new(root.path().to_path_buf()).unwrap();
    let mut app = App::with_panels(panel.clone(), panel);
    app.config.lsp.0.clear();
    let entry = Menu::open(Heading::Commands)
        .entries()
        .iter()
        .find(|entry| crate::menu::label_of(entry).contains("Language servers"))
        .copied()
        .expect("LSP menu entry");
    crate::ops::menu::activate(&mut app, entry);
    let Overlay::Viewer(viewer) = app.overlay else {
        panic!("expected inventory")
    };
    assert!(viewer
        .lines
        .iter()
        .any(|line| line.contains("Configured servers")));
}

#[test]
#[ignore = "explicit review artifact capture with synthetic installation paths"]
fn capture_lsp_inventory_review_screen() {
    use ratatui::{backend::TestBackend, Terminal};
    use serde_json::json;
    let panel = || Panel::from_entries("/workspace".into(), Vec::new());
    let mut app = App::with_panels(panel(), panel());
    let lines = inventory_lines();
    app.set_overlay(Overlay::Viewer(crate::models::Viewer::report(
        "Language servers",
        lines,
    )));
    let mut terminal = Terminal::new(TestBackend::new(110, 22)).unwrap();
    terminal
        .draw(|frame| crate::ui::ui(frame, &mut app))
        .unwrap();
    let cells: Vec<_> = terminal.backend().buffer().content.iter().map(|cell| json!({"text":cell.symbol(), "fg":cell.fg.to_string(), "bg":cell.bg.to_string()})).collect();
    println!(
        "FFM_SCREEN:{}",
        json!({"width":110,"height":22,"cells":cells})
    );
}

fn inventory_lines() -> Vec<String> {
    [
        "Configured servers; availability does not mean a server is running.",
        "Settings: C:/Users/bob/AppData/Roaming/ffm/settings.yaml",
        "Open a source file with F4 to start its server. Esc closes this report.",
        "",
        "python [enabled] .py .pyi",
        "  Configured command: pylsp",
        "  Configured arguments: (none)",
        "  Status: installed in FFM cache",
        "  Executable: C:/Users/bob/AppData/Roaming/ffm/lsp/<recipe>/venv/Scripts/python.exe",
        "  Launch argument: -m",
        "  Launch argument: pylsp",
        "  Auto-install: python-lsp-server[pyflakes,pycodestyle]",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}
