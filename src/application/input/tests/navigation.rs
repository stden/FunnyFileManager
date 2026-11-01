//! Read ahead only through navigation; commands and read failures keep their order.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use super::*;
use crate::models::{FolderTree, PanelKind, ViewMode};

#[test]
fn navigation_stops_before_commands_and_retains_errors() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = app_at(temp.path());
    app.close_overlay();
    let mut input = BufferedInput::new(Queue(VecDeque::from([
        Ok(key(KeyCode::Down)),
        Ok(key(KeyCode::F(8))),
        Err(io::Error::other("lost")),
    ])));
    assert_eq!(input.next_navigation(&app), Some(key(KeyCode::Down)));
    assert_eq!(input.next_navigation(&app), None);
    assert_eq!(input.read(&app)?, Some(key(KeyCode::F(8))));
    assert_eq!(input.next_navigation(&app), None);
    assert_eq!(input.read(&app).unwrap_err().to_string(), "lost");
    Ok(())
}

#[test]
fn navigation_does_not_read_through_overlays_or_resize() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = app_at(temp.path());
    let mut input = queued([key(KeyCode::Down), Event::Resize(40, 10)]);
    assert_eq!(input.next_navigation(&app), None);
    app.close_overlay();
    assert_eq!(input.next_navigation(&app), Some(key(KeyCode::Down)));
    assert_eq!(input.next_navigation(&app), None);
    assert_eq!(input.read(&app)?, Some(Event::Resize(40, 10)));
    assert_eq!(input.next_navigation(&app), None);
    Ok(())
}

#[test]
fn navigation_uses_configured_actions_instead_of_physical_keys() -> io::Result<()> {
    use crate::action::{Action, KeyChord};
    let temp = TempDir::new()?;
    let mut app = app_at(temp.path());
    app.close_overlay();
    let down = KeyChord::from_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)).unwrap();
    let letter =
        KeyChord::from_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)).unwrap();
    app.keymap.panels.bind(Action::DeleteConfirm, down);
    app.keymap.panels.bind(Action::NavDown, letter);
    let mut input = queued([key(KeyCode::Char('j')), key(KeyCode::Down)]);
    assert_eq!(input.next_navigation(&app), Some(key(KeyCode::Char('j'))));
    assert_eq!(input.next_navigation(&app), None);
    assert_eq!(input.read(&app)?, Some(key(KeyCode::Down)));
    Ok(())
}

#[test]
fn wheel_and_paging_keys_are_taken_in_order() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = app_at(temp.path());
    app.close_overlay();
    let wheel = Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollUp,
        column: 1,
        row: 1,
        modifiers: KeyModifiers::NONE,
    });
    let mut input = queued([
        wheel.clone(),
        key(KeyCode::PageDown),
        key(KeyCode::Home),
        key(KeyCode::End),
        key(KeyCode::PageUp),
    ]);
    assert_eq!(input.next_navigation(&app), Some(wheel));
    assert_eq!(input.next_navigation(&app), Some(key(KeyCode::PageDown)));
    assert_eq!(input.next_navigation(&app), Some(key(KeyCode::Home)));
    assert_eq!(input.next_navigation(&app), Some(key(KeyCode::End)));
    assert_eq!(input.next_navigation(&app), Some(key(KeyCode::PageUp)));
    assert_eq!(input.next_navigation(&app), None);
    Ok(())
}

#[test]
fn a_click_and_a_key_release_stay_for_the_normal_reader() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = app_at(temp.path());
    app.close_overlay();
    let click = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 2,
        row: 3,
        modifiers: KeyModifiers::NONE,
    });
    let released = Event::Key(KeyEvent::new_with_kind(
        KeyCode::Down,
        KeyModifiers::NONE,
        KeyEventKind::Release,
    ));
    let mut input = queued([click.clone(), released.clone()]);
    assert_eq!(input.next_navigation(&app), None);
    assert_eq!(input.read(&app)?, Some(click));
    assert_eq!(input.next_navigation(&app), None);
    assert_eq!(input.read(&app)?, Some(released));
    Ok(())
}

#[test]
fn console_view_and_a_tree_panel_do_not_batch_down() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = app_at(temp.path());
    app.close_overlay();
    app.view = ViewMode::Console;
    let mut input = queued([key(KeyCode::Down)]);
    assert_eq!(input.next_navigation(&app), None);
    app.view = ViewMode::Panels;
    let tree = FolderTree::rooted_at(temp.path().to_path_buf());
    app.left_panel.kind = PanelKind::Tree(Box::new(tree));
    assert_eq!(input.next_navigation(&app), None);
    assert_eq!(input.read(&app)?, Some(key(KeyCode::Down)));
    Ok(())
}
