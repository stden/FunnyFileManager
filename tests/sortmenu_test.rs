//! Purpose: Integration tests for Far's sort modes — `Ctrl+F12` and the direct
//! `Ctrl+F3` … `Ctrl+F11` keys — driven through the public API against a real
//! directory and a real rendered frame.

use std::fs;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::action::{resolve, Action, PanelCommand};
use funny_file_manager::menu::MenuCommand;
use funny_file_manager::models::{App, Overlay, Panel, SortMode};
use funny_file_manager::ops::sortmenu;
use funny_file_manager::popup::process_popup_input;
use funny_file_manager::ui::ui;
use ratatui::{backend::TestBackend, Terminal};
use tempfile::TempDir;

/// Files whose name order and size order disagree.
#[expect(
    clippy::expect_used,
    reason = "test fixture: a directory that cannot be set up fails the test that asked for it"
)]
fn app_over(temp: &TempDir) -> App {
    fs::write(temp.path().join("alpha.txt"), "a").expect("test setup");
    fs::write(temp.path().join("beta.txt"), "bbbbbbbb").expect("test setup");
    fs::write(temp.path().join("gamma.txt"), "ggg").expect("test setup");
    let left = Panel::new(temp.path().to_path_buf()).expect("test setup");
    let right = Panel::new(temp.path().to_path_buf()).expect("test setup");
    App::with_panels(left, right)
}

fn files(app: &App) -> Vec<String> {
    app.get_active_panel_ref()
        .entries
        .iter()
        .filter(|entry| !entry.kind.is_dir())
        .map(|entry| entry.name.clone())
        .collect()
}

fn ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

fn screen_text(app: &mut App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("test terminal");
    terminal.draw(|f| ui(f, app)).expect("draw");
    let buffer = terminal.backend().buffer();
    let mut text = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            text.push_str(buffer[(x, y)].symbol());
        }
        text.push('\n');
    }
    text
}

#[test]
fn ctrl_f12_opens_the_menu_and_the_ctrl_f_keys_name_fars_modes() {
    assert!(matches!(
        resolve(ctrl(KeyCode::F(12))),
        Action::MenuCommand(MenuCommand::Sort)
    ));
    let expected = [
        (3, SortMode::Name),
        (4, SortMode::Extension),
        (5, SortMode::ModificationTime),
        (6, SortMode::Size),
        (7, SortMode::Unsorted),
        (8, SortMode::CreationTime),
        (9, SortMode::AccessTime),
        (10, SortMode::Descriptions),
        (11, SortMode::Owner),
    ];
    for (key, mode) in expected {
        let action = resolve(ctrl(KeyCode::F(key)));
        assert!(
            matches!(action, Action::Panel(PanelCommand::Sort(found)) if found == mode),
            "Ctrl+F{key} -> {mode:?}, got {action:?}"
        );
    }
}

/// The key's whole life: a new mode re-lists, the same key again reverses.
#[test]
fn a_direct_key_relists_and_pressed_again_reverses() {
    let temp = TempDir::new().expect("test setup");
    let mut app = app_over(&temp);
    assert_eq!(files(&app), ["alpha.txt", "beta.txt", "gamma.txt"]);

    sortmenu::set(&mut app, SortMode::Size);
    assert_eq!(files(&app), ["beta.txt", "gamma.txt", "alpha.txt"]);

    sortmenu::set(&mut app, SortMode::Size);
    assert_eq!(files(&app), ["alpha.txt", "gamma.txt", "beta.txt"]);
    assert!(app.get_active_panel_ref().sort_reversed);
    assert_eq!(app.status.to_string(), "Sort: Size, reverse order");
}

/// Ctrl+F12, Down twice to "Modification time", Enter: the panel re-lists
/// by that mode and the menu is gone.
#[test]
fn choosing_a_row_from_the_menu_relists_the_panel() {
    let temp = TempDir::new().expect("test setup");
    let mut app = app_over(&temp);

    sortmenu::open(&mut app);
    assert!(matches!(app.overlay, Overlay::SortModes(_)));
    process_popup_input(&mut app, plain(KeyCode::Down));
    process_popup_input(&mut app, plain(KeyCode::Down));
    process_popup_input(&mut app, plain(KeyCode::Enter));

    assert!(!app.overlay.is_active());
    assert_eq!(
        app.get_active_panel_ref().sort_mode,
        SortMode::ModificationTime
    );
    assert_eq!(files(&app).len(), 3, "still the same files");
}

/// The rendered menu ticks the mode the panel is in and the reverse row when
/// the panel runs the other way; the panel behind it shows the mode letter.
#[test]
fn the_rendered_menu_ticks_the_panels_mode_and_order() {
    let temp = TempDir::new().expect("test setup");
    let mut app = app_over(&temp);
    sortmenu::set(&mut app, SortMode::Extension);
    process_popup_input(&mut app, plain(KeyCode::Esc));
    sortmenu::open(&mut app);
    process_popup_input(&mut app, plain(KeyCode::Char('*')));
    assert!(app.get_active_panel_ref().sort_reversed);

    sortmenu::open(&mut app);
    let screen = screen_text(&mut app);
    let ticked: Vec<&str> = screen.lines().filter(|line| line.contains('√')).collect();
    assert_eq!(ticked.len(), 2, "{screen}");
    assert!(ticked[0].contains("Ext"), "{}", ticked[0]);
    assert!(ticked[1].contains("Reverse order"), "{}", ticked[1]);
    assert!(screen.contains("Sort by"), "{screen}");
}

#[test]
fn escape_leaves_the_panel_exactly_as_it_was() {
    let temp = TempDir::new().expect("test setup");
    let mut app = app_over(&temp);
    let before = files(&app);

    sortmenu::open(&mut app);
    process_popup_input(&mut app, plain(KeyCode::End));
    process_popup_input(&mut app, plain(KeyCode::Esc));

    assert!(!app.overlay.is_active());
    assert_eq!(files(&app), before);
    assert!(!app.get_active_panel_ref().sort_reversed);
}
