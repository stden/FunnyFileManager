//! Unit tests: where exactly the `NetBox` panels and dialogs put things —
//! the heading row, each list row, the info line, the frame — and which
//! control carries the focus colour.

use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::Terminal;
use tempfile::TempDir;

use super::{palette, render_overlays, FAR_BLUE};
use crate::models::{
    App, AuthKind, DirEntry, EntryKind, InputPrompt, InputPurpose, Overlay, PanelKind,
    PluginCommand, PluginCommands, Protocol, Question, QuestionPurpose, RemotePanel, RemotePath,
    RowMotion, SessionEdit,
};
use crate::remote::fake::FakeRemoteFs;
use crate::remote::RemoteLink;
use crate::sessions::SessionStore;
use crate::test_support::netbox::{app_in, app_on_sessions, name, session};
use crate::test_support::{buffer_text, WritableTestBackend};

/// A 40 by 10 panel: the frame, a heading, six rows, the info line.
const PANEL: Rect = Rect::new(0, 0, 40, 10);

type Screen = (Vec<String>, Terminal<WritableTestBackend>);

fn lines(terminal: &Terminal<WritableTestBackend>) -> Vec<String> {
    buffer_text(terminal).lines().map(str::to_owned).collect()
}

fn draw_sessions(app: &mut App, is_active: bool) -> Screen {
    let mut terminal = Terminal::new(WritableTestBackend::new(40, 10)).expect("terminal");
    terminal
        .draw(|f| super::sessions::render(f, &mut app.left_panel, PANEL, is_active))
        .expect("draw");
    (lines(&terminal), terminal)
}

fn draw_remote(app: &mut App, is_active: bool) -> Screen {
    let mut terminal = Terminal::new(WritableTestBackend::new(40, 10)).expect("terminal");
    terminal
        .draw(|f| super::remote::render(f, &mut app.left_panel, PANEL, is_active))
        .expect("draw");
    (lines(&terminal), terminal)
}

fn sessions_app(dir: &TempDir, hosts: &[&str]) -> App {
    let mut store = SessionStore::default();
    for host in hosts {
        store.upsert(session(host, None));
    }
    app_on_sessions(dir.path(), store)
}

fn move_sessions(app: &mut App, motion: RowMotion) {
    if let PanelKind::Sessions(panel) = &mut app.left_panel.kind {
        panel.move_cursor(motion);
    }
}

#[test]
fn the_sessions_panel_puts_each_part_on_its_row() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = sessions_app(&dir, &["a.test", "b.test"]);
    let (rows, _) = draw_sessions(&mut app, true);
    assert!(
        rows[0].contains(" NetBox: Stored sessions [/] "),
        "{rows:?}"
    );
    assert!(rows[1].starts_with("║Session name"), "{rows:?}");
    assert!(rows[2].starts_with("║.. "), "{rows:?}");
    assert!(rows[3].starts_with("║a.test "), "{rows:?}");
    assert!(rows[4].starts_with("║b.test "), "{rows:?}");
    assert!(rows[5].trim_matches(['║', ' ']).is_empty(), "{rows:?}");
    assert!(rows[8].starts_with("║2 sessions "), "{rows:?}");
    assert!(
        rows[9].starts_with('╚') && rows[9].ends_with('╝'),
        "{rows:?}"
    );
}

#[test]
fn a_long_list_shows_its_last_six_rows_under_the_heading() {
    let dir = TempDir::new().expect("temp dir");
    let hosts: Vec<String> = (0..12).map(|index| format!("h{index:02}.test")).collect();
    let hosts: Vec<&str> = hosts.iter().map(String::as_str).collect();
    let mut app = sessions_app(&dir, &hosts);
    move_sessions(&mut app, RowMotion::Last);
    let (rows, _) = draw_sessions(&mut app, true);
    assert!(rows[1].starts_with("║Session name"), "{rows:?}");
    assert!(rows[2].starts_with("║h06.test"), "{rows:?}");
    assert!(rows[7].starts_with("║h11.test"), "{rows:?}");
    assert!(rows[8].starts_with("║12 sessions"), "{rows:?}");
    let PanelKind::Sessions(panel) = &app.left_panel.kind else {
        panic!("the sessions list");
    };
    assert_eq!(panel.scroll, 7);
}

#[test]
fn the_rows_are_coloured_by_kind_and_the_cursor_marks_the_active_panel_only() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = sessions_app(&dir, &["a.test", "b.test"]);
    move_sessions(&mut app, RowMotion::Next);
    let (_, terminal) = draw_sessions(&mut app, true);
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(1, 2)].fg, palette::DIRECTORY);
    assert!(buffer[(1, 2)].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer[(1, 3)].bg, palette::CURSOR_BG);
    assert_eq!(buffer[(1, 3)].fg, palette::CURSOR_FG);
    assert_eq!(buffer[(1, 4)].fg, palette::FILE);
    assert_ne!(buffer[(1, 4)].bg, palette::CURSOR_BG);

    let (_, terminal) = draw_sessions(&mut app, false);
    assert_ne!(terminal.backend().buffer()[(1, 3)].bg, palette::CURSOR_BG);
}

#[test]
fn the_active_panels_frame_is_bold_and_its_title_highlighted() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = sessions_app(&dir, &["a.test"]);
    let (rows, terminal) = draw_sessions(&mut app, true);
    let title = rows[0].find("NetBox").expect("the title");
    let title = u16::try_from(rows[0][..title].chars().count()).expect("fits");
    let buffer = terminal.backend().buffer();
    assert!(buffer[(0, 5)].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer[(0, 5)].fg, palette::FRAME);
    assert_eq!(buffer[(title, 0)].bg, palette::HIGHLIGHT_BG);
    assert_eq!(buffer[(title, 0)].fg, palette::HIGHLIGHT_FG);

    let (_, terminal) = draw_sessions(&mut app, false);
    let buffer = terminal.backend().buffer();
    assert!(!buffer[(0, 5)].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer[(title, 0)].bg, FAR_BLUE);
    assert_eq!(buffer[(1, 1)].fg, palette::HOTKEY, "the heading");
    assert_eq!(buffer[(1, 8)].fg, palette::DATA, "the info line");
}

fn remote_app(dir: &TempDir) -> App {
    let entry = |name: &str, kind| DirEntry {
        name: name.to_owned(),
        kind,
        ..DirEntry::default()
    };
    let mut panel = RemotePanel::new(
        name("demo"),
        RemoteLink::new(Box::new(FakeRemoteFs::default())),
    );
    let _ = panel.show(
        RemotePath::parse("/var/www").expect("path"),
        vec![
            entry("html", EntryKind::Directory),
            entry("a.txt", EntryKind::File),
        ],
    );
    let mut app = app_in(dir.path());
    app.left_panel.kind = PanelKind::Remote(Box::new(panel));
    app
}

#[test]
fn the_remote_panel_puts_each_part_on_its_row_and_marks_the_cursor() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = remote_app(&dir);
    if let PanelKind::Remote(panel) = &mut app.left_panel.kind {
        panel.move_cursor(RowMotion::Next);
    }
    let (rows, terminal) = draw_remote(&mut app, true);
    assert!(rows[0].contains(" demo:/var/www "), "{rows:?}");
    assert!(rows[1].starts_with("║Name"), "{rows:?}");
    assert!(rows[2].starts_with("║.. "), "{rows:?}");
    assert!(rows[3].starts_with("║html "), "{rows:?}");
    assert!(rows[4].starts_with("║a.txt "), "{rows:?}");
    assert!(rows[8].starts_with("║/var/www "), "{rows:?}");
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(1, 3)].bg, palette::CURSOR_BG);
    assert_ne!(buffer[(1, 2)].bg, palette::CURSOR_BG);
    assert_ne!(buffer[(1, 4)].bg, palette::CURSOR_BG);

    let (_, terminal) = draw_remote(&mut app, false);
    assert_ne!(terminal.backend().buffer()[(1, 3)].bg, palette::CURSOR_BG);
}

fn draw_overlay(app: &mut App) -> (String, Terminal<WritableTestBackend>) {
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");
    terminal.draw(|f| render_overlays(f, app)).expect("draw");
    (buffer_text(&terminal), terminal)
}

/// The cell where `text` starts on `screen`.
fn at(screen: &str, text: &str) -> (u16, u16) {
    screen
        .lines()
        .enumerate()
        .find_map(|(row, line)| {
            line.find(text).map(|byte| {
                let column = u16::try_from(line[..byte].chars().count()).expect("fits");
                (column, u16::try_from(row).expect("fits"))
            })
        })
        .unwrap_or_else(|| panic!("{text:?} on\n{screen}"))
}

#[test]
fn the_plugin_list_is_as_wide_as_its_longest_row_plus_padding_and_frame() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.set_overlay(Overlay::PluginCommands(PluginCommands::default()));
    let _ = draw_overlay(&mut app);
    let longest = PluginCommand::ImportNetBox.label().chars().count();
    let expected = super::drives::centered(
        Rect::new(0, 0, 80, 24),
        1 + longest + 1 + 2,
        PluginCommand::ALL.len() + 2,
    );
    assert_eq!(app.dialog_mouse.area, Some(expected));
}

#[test]
fn the_focused_button_carries_the_focus_colour() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let mut prompt = InputPrompt::new(InputPurpose::ImportNetBox, "");
    app.set_overlay(Overlay::InputPrompt(Box::new(prompt.clone())));
    let (screen, terminal) = draw_overlay(&mut app);
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[at(&screen, "OK")].bg, palette::DIALOG_FOCUS_BG);
    assert_ne!(buffer[at(&screen, "Cancel")].bg, palette::DIALOG_FOCUS_BG);

    prompt.toggle_button();
    app.set_overlay(Overlay::InputPrompt(Box::new(prompt)));
    let (screen, terminal) = draw_overlay(&mut app);
    let buffer = terminal.backend().buffer();
    assert_ne!(buffer[at(&screen, "OK")].bg, palette::DIALOG_FOCUS_BG);
    assert_eq!(buffer[at(&screen, "Cancel")].bg, palette::DIALOG_FOCUS_BG);
}

#[test]
fn the_questions_answer_under_the_cursor_carries_the_focus_colour() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let mut question = Question::new(QuestionPurpose::DeleteSession(name("web")));
    app.set_overlay(Overlay::Question(question.clone()));
    let (screen, terminal) = draw_overlay(&mut app);
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[at(&screen, "Yes")].bg, palette::DIALOG_FOCUS_BG);
    assert_ne!(buffer[at(&screen, "No ")].bg, palette::DIALOG_FOCUS_BG);

    question.toggle();
    app.set_overlay(Overlay::Question(question));
    let (screen, terminal) = draw_overlay(&mut app);
    let buffer = terminal.backend().buffer();
    assert_ne!(buffer[at(&screen, "Yes")].bg, palette::DIALOG_FOCUS_BG);
    assert_eq!(buffer[at(&screen, "No ")].bg, palette::DIALOG_FOCUS_BG);
}

#[test]
fn the_session_dialog_marks_the_chosen_protocol_and_login() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let mut dialog = SessionEdit::create();
    dialog.protocol = Protocol::Ftps;
    dialog.auth = AuthKind::Agent;
    app.set_overlay(Overlay::SessionEdit(Box::new(dialog)));
    let (screen, _) = draw_overlay(&mut app);
    for text in ["(•) FTPS", "( ) SFTP", "(•) Agent", "( ) Password"] {
        assert!(screen.contains(text), "{text:?} in\n{screen}");
    }
}

#[test]
fn the_session_dialogs_buttons_take_the_focus_in_turn() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let mut dialog = SessionEdit::create();
    for _ in 0..7 {
        dialog.tab(true);
    }
    assert_eq!(dialog.field(), crate::models::SessionField::Ok);
    app.set_overlay(Overlay::SessionEdit(Box::new(dialog.clone())));
    let (screen, terminal) = draw_overlay(&mut app);
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[at(&screen, "Ok ")].bg, palette::DIALOG_FOCUS_BG);
    assert_ne!(buffer[at(&screen, "Cancel")].bg, palette::DIALOG_FOCUS_BG);

    dialog.tab(true);
    app.set_overlay(Overlay::SessionEdit(Box::new(dialog)));
    let (screen, terminal) = draw_overlay(&mut app);
    let buffer = terminal.backend().buffer();
    assert_ne!(buffer[at(&screen, "Ok ")].bg, palette::DIALOG_FOCUS_BG);
    assert_eq!(buffer[at(&screen, "Cancel")].bg, palette::DIALOG_FOCUS_BG);
}

/// Typing lands in the focused field, so the caret stands at its end.
#[test]
fn the_caret_stands_at_the_end_of_the_focused_field() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let mut dialog = SessionEdit::create();
    dialog.name = "web".to_owned();
    dialog.tab(true);
    dialog.tab(true);
    dialog.host = "example.test".to_owned();
    assert_eq!(dialog.field(), crate::models::SessionField::Host);
    app.set_overlay(Overlay::SessionEdit(Box::new(dialog)));
    let (screen, mut terminal) = draw_overlay(&mut app);
    let (x, y) = at(&screen, "example.test");
    let caret = terminal.get_cursor_position().expect("cursor");
    assert_eq!((caret.x, caret.y), (x + 12, y));
}
