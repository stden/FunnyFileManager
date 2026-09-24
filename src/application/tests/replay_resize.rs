//! Replay geometry determines both drawing and mouse hit testing.
use super::*;
use crate::application::event::handle_event;
use crate::application::loop_state::draw_frame;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::{layout::Rect, TerminalOptions, Viewport as TerminalViewport};

#[test]
fn recorded_resize_updates_the_frame_and_mouse_command_row() {
    let root = tempfile::TempDir::new().expect("directory");
    let mut app = App::new().expect("app");
    app.left_panel = models::Panel::new(root.path().to_path_buf()).expect("panel");
    app.right_panel = models::Panel::new(root.path().to_path_buf()).expect("panel");
    let options = TerminalOptions {
        viewport: TerminalViewport::Fixed(Rect::new(0, 0, 80, 24)),
    };
    let mut terminal =
        Terminal::with_options(WritableTestBackend::new(100, 40), options).expect("terminal");
    let mut clicks = crate::mouse::PanelClicks::default();
    handle_event(&mut terminal, &mut app, Event::Resize(60, 18), &mut clicks).expect("resize");
    draw_frame(&mut terminal, &mut app).expect("draw");
    assert_eq!(terminal.get_frame().area(), Rect::new(0, 0, 60, 18));
    let click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: 16,
        modifiers: KeyModifiers::NONE,
    };
    handle_event(&mut terminal, &mut app, Event::Mouse(click), &mut clicks).expect("mouse");
    assert!(matches!(app.overlay, Overlay::CommandLine(_)));
}
