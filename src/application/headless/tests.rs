//! Purpose: Unit tests for headless runtime viewport synchronization, input batching and actions.

use super::*;
use crate::{
    action::{Action, KeyChord},
    models::PanelSide,
    terminal::headless::HeadlessHost,
};
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use tempfile::TempDir;

fn key_event(chord: &str) -> Event {
    Event::Key(chord.parse::<KeyChord>().expect("valid chord").to_event())
}

fn mouse_click(column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

#[test]
fn pump_resizes_memory_terminal_when_headless_viewport_changes() -> io::Result<()> {
    let root = TempDir::new()?;
    let host = HeadlessHost::new();
    let _scope = host.enter();
    let mut runtime = Runtime::new(root.path(), Size::new(80, 25), Profile::Isolated)?;
    let _ = runtime.pump()?;

    crate::terminal::headless::resize(104, 38)?;
    assert!(runtime.pump()?);
    assert_eq!(
        runtime.terminal.backend().buffer().area,
        Rect::new(0, 0, 104, 38)
    );
    Ok(())
}

#[test]
fn runtime_resize_updates_headless_viewport_and_memory_buffer() -> io::Result<()> {
    let root = TempDir::new()?;
    let host = HeadlessHost::new();
    let _scope = host.enter();
    let mut runtime = Runtime::new(root.path(), Size::new(80, 24), Profile::Isolated)?;

    runtime.resize(Size::new(96, 32))?;
    assert_eq!(
        crate::terminal::headless::viewport(),
        Some(Size::new(96, 32))
    );
    assert_eq!(
        runtime.terminal.backend().buffer().area,
        Rect::new(0, 0, 96, 32)
    );
    Ok(())
}

#[test]
fn input_flushes_layout_around_mouse_events_and_processes_keys() -> io::Result<()> {
    let root = TempDir::new()?;
    std::fs::write(root.path().join("alpha.txt"), b"a")?;
    std::fs::write(root.path().join("beta.txt"), b"b")?;
    let host = HeadlessHost::new();
    let _scope = host.enter();
    let mut runtime = Runtime::new(root.path(), Size::new(100, 30), Profile::Isolated)?;

    runtime.input(vec![
        key_event("Down"),
        mouse_click(75, 4),
        key_event("Tab"),
    ])?;
    assert_eq!(runtime.app.active_panel, PanelSide::Left);
    Ok(())
}

#[test]
fn invoke_dispatches_canonical_action_and_redraws() -> io::Result<()> {
    let root = TempDir::new()?;
    let host = HeadlessHost::new();
    let _scope = host.enter();
    let mut runtime = Runtime::new(root.path(), Size::new(80, 25), Profile::Isolated)?;
    assert_eq!(runtime.app.active_panel, PanelSide::Left);

    runtime.invoke(Invocation::Panels(Action::SwitchPanel))?;
    assert_eq!(runtime.app.active_panel, PanelSide::Right);
    Ok(())
}
