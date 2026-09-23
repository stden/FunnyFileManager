//! Unit tests: the Install MCP server dialog draws the server, the scope, the
//! client rows and their notes on an 80×24 screen, dims what cannot be
//! changed, and records where it landed for the mouse.

use ratatui::Terminal;

use crate::mcp_clients::{ClientId, Scope};
use crate::models::{App, McpInstallDialog, McpInstallField, Overlay, Panel};
use crate::test_support::{buffer_text, FakeMachine, WritableTestBackend};
use crate::ui::render_overlays;

fn machine() -> FakeMachine {
    let machine = FakeMachine::new().expect("fake machine");
    machine.install_clients().unwrap();
    machine
}

fn opened(machine: &FakeMachine) -> App {
    let mut app = App::with_panels(
        Panel::new(machine.folder()).unwrap(),
        Panel::new(machine.folder()).unwrap(),
    );
    app.live.mcp_host = machine.host();
    crate::ops::mcp_install::open(&mut app);
    app
}

fn draw(app: &mut App) -> (String, Terminal<WritableTestBackend>) {
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");
    terminal.draw(|f| render_overlays(f, app)).expect("draw");
    (buffer_text(&terminal), terminal)
}

fn dialog_mut(app: &mut App) -> &mut McpInstallDialog {
    match &mut app.overlay {
        Overlay::McpInstall(dialog) => dialog,
        _ => panic!("the dialog is not open"),
    }
}

#[test]
fn the_dialog_shows_every_control_at_all_projects() {
    let machine = machine();
    machine.install_server().unwrap();
    let mut app = opened(&machine);
    let (screen, _) = draw(&mut app);
    for text in [
        "Install MCP server",
        "Server: ",
        "Scope: (•) All projects( ) Current folder",
        "Current folder: ",
        "[x] Claude Code  detected      — no config",
        "[x] Codex        detected",
        "Junie: may start ffm-mcp outside the project; prefer Current folder.",
        "Kiro: may start ffm-mcp outside the project; prefer Current folder.",
        "{ Register }",
        "[ Cancel ]",
    ] {
        assert!(screen.contains(text), "missing {text:?}\n{screen}");
    }
    assert!(app.dialog_mouse.area.is_some(), "the frame is recorded");
    assert_eq!(app.dialog_mouse.buttons.len(), 2);
}

#[test]
fn the_current_folder_shows_its_own_notes() {
    let machine = machine();
    machine.install_server().unwrap();
    let mut app = opened(&machine);
    crate::ops::mcp_install::switch_scope(&mut app, crate::search::Direction::Forward);
    let (screen, _) = draw(&mut app);
    assert!(
        screen.contains("( ) All projects(•) Current folder"),
        "{screen}"
    );
    assert!(
        screen.contains("Codex: reads .codex/config.toml only in folders it trusts."),
        "{screen}"
    );
    assert!(!screen.contains("Junie: may start"), "{screen}");
}

#[test]
fn a_missing_server_is_explained_and_register_is_dimmed() {
    let machine = machine();
    let mut app = opened(&machine);
    let (screen, terminal) = draw(&mut app);
    assert!(
        screen.contains("Server: ffm-mcp was not found next to ffm or on PATH."),
        "{screen}"
    );
    assert!(screen.contains("cargo install --path mcp"), "{screen}");
    assert!(screen.contains("[ Register ]"), "{screen}");
    assert!(screen.contains("{ Cancel }"), "{screen}");
    let (x, y) = find(&screen, "[ ] Claude Code");
    let cell = &terminal.backend().buffer()[(x, y)];
    assert_eq!(
        cell.fg,
        crate::ui::palette::DISABLED_FG,
        "a disabled row is dimmed"
    );
}

#[test]
fn a_folder_that_is_not_local_is_said_so() {
    let machine = machine();
    machine.install_server().unwrap();
    let mut app = opened(&machine);
    dialog_mut(&mut app).folder = None;
    let (screen, _) = draw(&mut app);
    assert!(
        screen.contains("Current folder: the active panel is not a local folder"),
        "{screen}"
    );
}

#[test]
fn the_focused_row_and_button_follow_the_field() {
    let machine = machine();
    machine.install_server().unwrap();
    let mut app = opened(&machine);
    dialog_mut(&mut app).field = McpInstallField::Client(ClientId::Codex);
    let (screen, terminal) = draw(&mut app);
    let (x, y) = find(&screen, "[x] Codex");
    let cell = &terminal.backend().buffer()[(x, y)];
    assert_eq!(cell.bg, crate::ui::palette::DIALOG_FOCUS_BG);
    dialog_mut(&mut app).scope = Scope::CurrentFolder;
    dialog_mut(&mut app).field = McpInstallField::Register;
    let (screen, _) = draw(&mut app);
    assert!(screen.contains("(•) Current folder"), "{screen}");
}

/// The cell where `text` starts on the drawn screen.
fn find(screen: &str, text: &str) -> (u16, u16) {
    screen
        .lines()
        .enumerate()
        .find_map(|(row, line)| {
            line.split_once(text).map(|(before, _)| {
                let column = before.chars().count();
                (u16::try_from(column).unwrap(), u16::try_from(row).unwrap())
            })
        })
        .unwrap_or_else(|| panic!("{text:?} is not on screen\n{screen}"))
}

/// Review finding: `&` in a path was taken for a hotkey marker, so
/// `/srv/x/R&D` was drawn as `/srv/x/RD` with the `D` lit up.
#[test]
fn an_ampersand_in_a_path_is_drawn_as_it_is() {
    let machine = machine();
    machine.install_server().unwrap();
    let folder = machine.folder().join("R&D");
    std::fs::create_dir(&folder).unwrap();
    let mut app = App::with_panels(
        Panel::new(folder.clone()).unwrap(),
        Panel::new(folder).unwrap(),
    );
    app.live.mcp_host = machine.host();
    crate::ops::mcp_install::open(&mut app);
    let (screen, _) = draw(&mut app);
    assert!(screen.contains("R&D"), "{screen}");
}
