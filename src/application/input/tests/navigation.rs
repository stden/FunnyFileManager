//! Read ahead only through navigation; commands and read failures keep their order.

use super::*;

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
