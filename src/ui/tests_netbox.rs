//! Unit tests: the `NetBox` panels and dialogs draw what their state says —
//! titles, headings, rows, the cursor bar, the info line — and a name from a
//! host is shown, never obeyed.

use ratatui::Terminal;
use tempfile::TempDir;

use super::palette;
use super::{render_overlays, ui};
use crate::models::{
    App, DirEntry, EntryKind, InputPrompt, InputPurpose, Overlay, PanelKind, PluginCommands,
    Question, QuestionPurpose, RemotePanel, RemotePath, RowMotion, SessionEdit, SessionsPanel,
};
use crate::remote::fake::FakeRemoteFs;
use crate::remote::RemoteLink;
use crate::sessions::SessionStore;
use crate::test_support::netbox::{app_in, app_on_sessions, name, session};
use crate::test_support::{buffer_text, WritableTestBackend};

fn terminal(width: u16, height: u16) -> Terminal<WritableTestBackend> {
    Terminal::new(WritableTestBackend::new(width, height)).expect("test terminal")
}

fn draw(app: &mut App, width: u16, height: u16) -> (String, Terminal<WritableTestBackend>) {
    let mut terminal = terminal(width, height);
    terminal.draw(|f| ui(f, app)).expect("draw");
    (buffer_text(&terminal), terminal)
}

fn draw_overlay(app: &mut App) -> (String, Terminal<WritableTestBackend>) {
    let mut terminal = terminal(80, 24);
    terminal.draw(|f| render_overlays(f, app)).expect("draw");
    (buffer_text(&terminal), terminal)
}

/// The row of the screen holding `text`, and the column it starts in.
fn find(screen: &str, text: &str) -> (u16, u16) {
    screen
        .lines()
        .enumerate()
        .find_map(|(row, line)| {
            let (before, _) = line.split_once(text)?;
            Some((
                u16::try_from(before.chars().count()).expect("fits"),
                u16::try_from(row).expect("fits"),
            ))
        })
        .unwrap_or_else(|| panic!("{text:?} is on screen:\n{screen}"))
}

fn sessions_app(dir: &TempDir) -> App {
    let mut store = SessionStore::default();
    store.upsert(session("a.test", Some("demo")));
    let mut renamed = session("b.test", Some("demo"));
    renamed.name = name("web");
    store.upsert(renamed);
    app_on_sessions(dir.path(), store)
}

/// The Shift+F4 dialog with `host` typed in.
fn dialog_for(host: &str) -> SessionEdit {
    let mut dialog = SessionEdit::create();
    dialog.host = host.to_owned();
    dialog
}

#[test]
fn the_sessions_panel_draws_its_title_heading_rows_and_count() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = sessions_app(&dir);
    let (screen, _) = draw(&mut app, 100, 20);
    for text in [
        " NetBox: Stored sessions [/] ",
        "Session name",
        "..",
        "demo@a.test",
        "web  demo@b.test",
        "2 sessions",
    ] {
        assert!(screen.contains(text), "{text:?} in\n{screen}");
    }

    let mut single = app_on_sessions(dir.path(), {
        let mut store = SessionStore::default();
        store.upsert(session("a.test", None));
        store
    });
    let (screen, _) = draw(&mut single, 100, 20);
    assert!(screen.contains("1 session "), "{screen}");
}

#[test]
fn the_cursor_bar_marks_the_active_panels_row_only() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = sessions_app(&dir);
    if let PanelKind::Sessions(panel) = &mut app.left_panel.kind {
        panel.move_cursor(RowMotion::Next);
    }
    let (screen, terminal) = draw(&mut app, 100, 20);
    let (x, y) = find(&screen, "demo@a.test");
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(x, y)].bg, palette::CURSOR_BG);
    let (x, y) = find(&screen, "web  demo");
    assert_ne!(buffer[(x, y)].bg, palette::CURSOR_BG);

    app.active_panel = crate::models::PanelSide::Right;
    let (screen, terminal) = draw(&mut app, 100, 20);
    let (x, y) = find(&screen, "demo@a.test");
    assert_ne!(
        terminal.backend().buffer()[(x, y)].bg,
        palette::CURSOR_BG,
        "no bar on the inactive panel"
    );
}

#[test]
fn a_long_list_scrolls_to_keep_the_cursor_on_screen() {
    let dir = TempDir::new().expect("temp dir");
    let mut store = SessionStore::default();
    for index in 0..30 {
        store.upsert(session(&format!("h{index:02}.test"), None));
    }
    let mut app = app_on_sessions(dir.path(), store);
    if let PanelKind::Sessions(panel) = &mut app.left_panel.kind {
        panel.move_cursor(RowMotion::Last);
    }
    let (screen, _) = draw(&mut app, 100, 16);
    assert!(screen.contains("h29.test"), "{screen}");
    assert!(!screen.contains("h00.test"), "{screen}");
    let PanelKind::Sessions(panel) = &app.left_panel.kind else {
        panic!("the sessions list");
    };
    assert!(panel.scroll > 0, "the scroll is kept for the next frame");
}

#[test]
fn a_panel_too_short_for_a_row_draws_only_its_frame() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = sessions_app(&dir);
    let mut terminal = terminal(60, 4);
    let area = ratatui::layout::Rect::new(0, 0, 60, 4);
    terminal
        .draw(|f| super::sessions::render(f, &mut app.left_panel, area, true))
        .expect("draw");
    let screen = buffer_text(&terminal);
    assert!(screen.contains("NetBox: Stored sessions"), "{screen}");
    assert!(!screen.contains("Session name"), "{screen}");
}

fn remote_app(dir: &TempDir, listing: Vec<DirEntry>) -> App {
    let mut app = app_in(dir.path());
    let mut panel = RemotePanel::new(
        name("demo@example.test"),
        RemoteLink::new(Box::new(FakeRemoteFs::default())),
    );
    let shown = panel.show(RemotePath::parse("/var/www").expect("path"), listing);
    assert_eq!(
        shown.unusable_names, 0,
        "the fixture listing holds only usable names"
    );
    app.left_panel.kind = PanelKind::Remote(Box::new(panel));
    app
}

fn entry(name: &str, kind: EntryKind) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        kind,
        ..DirEntry::default()
    }
}

#[test]
fn the_remote_panel_draws_the_session_and_directory_and_its_entries() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = remote_app(
        &dir,
        vec![
            entry("html", EntryKind::Directory),
            entry("a.txt", EntryKind::File),
        ],
    );
    let (screen, terminal) = draw(&mut app, 100, 20);
    for text in [
        " demo@example.test:/var/www ",
        "Name",
        "html",
        "a.txt",
        "/var/www ",
    ] {
        assert!(screen.contains(text), "{text:?} in\n{screen}");
    }
    let (x, y) = find(&screen, "html");
    assert_eq!(terminal.backend().buffer()[(x, y)].fg, palette::DIRECTORY);
    let (x, y) = find(&screen, "a.txt");
    assert_eq!(terminal.backend().buffer()[(x, y)].fg, palette::FILE);
}

/// A host may send any name but `/`; one carrying an escape sequence is
/// shown with a mark where the escape was, not sent to the terminal.
#[test]
fn a_hosts_name_is_shown_and_never_obeyed() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = remote_app(&dir, vec![entry("evil\u{1b}[2Jname", EntryKind::File)]);
    let (screen, _) = draw(&mut app, 100, 20);
    assert!(screen.contains("evil\u{FFFD}[2Jname"), "{screen}");
}

#[test]
fn a_remote_panel_too_short_for_a_row_draws_only_its_frame() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = remote_app(&dir, Vec::new());
    let mut terminal = terminal(60, 3);
    let area = ratatui::layout::Rect::new(0, 0, 60, 3);
    terminal
        .draw(|f| super::remote::render(f, &mut app.left_panel, area, false))
        .expect("draw");
    let screen = buffer_text(&terminal);
    assert!(screen.contains("demo@example.test:/var/www"), "{screen}");
    assert!(!screen.contains("Name"), "{screen}");
}

#[test]
fn a_panel_of_another_kind_is_not_drawn_by_the_netbox_renderers() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let mut terminal = terminal(60, 10);
    let area = ratatui::layout::Rect::new(0, 0, 60, 10);
    terminal
        .draw(|f| {
            super::sessions::render(f, &mut app.left_panel, area, true);
            super::remote::render(f, &mut app.left_panel, area, true);
        })
        .expect("draw");
    assert!(buffer_text(&terminal).trim().is_empty());
}

#[test]
fn the_plugin_commands_list_draws_every_row_and_the_cursor() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let mut list = PluginCommands::default();
    list.move_cursor(RowMotion::Next);
    app.set_overlay(Overlay::PluginCommands(list));
    let (screen, terminal) = draw_overlay(&mut app);
    for text in [
        " Plugin commands ",
        "MCP status",
        "NetBox: stored sessions",
        "NetBox: import from .netbox file…",
    ] {
        assert!(screen.contains(text), "{text:?} in\n{screen}");
    }
    let (x, y) = find(&screen, "NetBox: stored");
    assert_eq!(terminal.backend().buffer()[(x, y)].bg, palette::CURSOR_BG);
    let (x, y) = find(&screen, "MCP status");
    assert_eq!(terminal.backend().buffer()[(x, y)].bg, palette::MENU_BG);
    assert!(
        app.dialog_mouse.area.is_some(),
        "the frame is a drag handle"
    );
}

#[test]
fn the_input_prompt_draws_its_purpose_and_puts_the_caret_in_the_text() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let mut prompt = InputPrompt::new(InputPurpose::ImportNetBox, "/srv/x/demo.netbox");
    prompt.input_mut().home();
    app.set_overlay(Overlay::InputPrompt(Box::new(prompt)));
    let mut terminal = terminal(80, 24);
    terminal
        .draw(|f| render_overlays(f, &mut app))
        .expect("draw");
    let screen = buffer_text(&terminal);
    for text in [
        "NetBox: import from .netbox file",
        "Read sessions from:",
        "/srv/x/demo.netbox",
        "OK",
        "Cancel",
    ] {
        assert!(screen.contains(text), "{text:?} in\n{screen}");
    }
    let (x, y) = find(&screen, "/srv/x/demo.netbox");
    let caret = terminal.get_cursor_position().expect("cursor");
    assert_eq!((caret.x, caret.y), (x, y), "the caret is at Home");
}

#[test]
fn the_question_draws_its_sentence_and_both_answers() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.set_overlay(Overlay::Question(Question::new(
        QuestionPurpose::DeleteSession(name("web")),
    )));
    let (screen, _) = draw_overlay(&mut app);
    for text in [
        "NetBox: delete session",
        "Delete the stored session web?",
        "Yes",
        "No",
    ] {
        assert!(screen.contains(text), "{text:?} in\n{screen}");
    }
}

const SESSION_DIALOG_TEXTS: [&str; 16] = [
    "New session",
    "Name",
    "Protocol",
    "SFTP",
    "FTPS",
    "Host",
    "example.test",
    "Port",
    "22",
    "User",
    "Login",
    "Agent",
    "Key file",
    "Directory",
    "Ok",
    "Cancel",
];

#[test]
fn the_session_dialog_draws_every_field_and_no_password() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.set_overlay(Overlay::SessionEdit(Box::new(dialog_for("example.test"))));
    let (screen, _) = draw_overlay(&mut app);
    for text in SESSION_DIALOG_TEXTS {
        assert!(screen.contains(text), "{text:?} in\n{screen}");
    }
    assert!(!screen.contains("Save password"), "{screen}");

    let stored = session("a.test", None);
    app.set_overlay(Overlay::SessionEdit(Box::new(SessionEdit::from_session(
        &stored,
    ))));
    let (screen, _) = draw_overlay(&mut app);
    assert!(screen.contains("Edit session"), "{screen}");
}

#[test]
fn an_empty_sessions_panel_still_offers_the_way_back() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.left_panel.kind = PanelKind::Sessions(Box::new(SessionsPanel::from_store(
        &SessionStore::default(),
    )));
    let (screen, _) = draw(&mut app, 80, 20);
    assert!(screen.contains(".."), "{screen}");
    assert!(screen.contains("0 sessions"), "{screen}");
}
