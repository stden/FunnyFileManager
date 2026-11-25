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

struct FailClearBackend(WritableTestBackend);

impl Write for FailClearBackend {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        Write::flush(&mut self.0)
    }
}

impl Backend for FailClearBackend {
    type Error = io::Error;
    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a ratatui::buffer::Cell)>,
    {
        self.0.draw(content)
    }
    fn hide_cursor(&mut self) -> io::Result<()> {
        self.0.hide_cursor()
    }
    fn show_cursor(&mut self) -> io::Result<()> {
        self.0.show_cursor()
    }
    fn get_cursor_position(&mut self) -> io::Result<ratatui::layout::Position> {
        self.0.get_cursor_position()
    }
    fn set_cursor_position<P: Into<ratatui::layout::Position>>(
        &mut self,
        pos: P,
    ) -> io::Result<()> {
        self.0.set_cursor_position(pos)
    }
    fn clear(&mut self) -> io::Result<()> {
        Err(io::Error::other("clear failed"))
    }
    fn clear_region(&mut self, _clear_type: ratatui::backend::ClearType) -> io::Result<()> {
        Err(io::Error::other("clear failed"))
    }
    fn size(&self) -> io::Result<ratatui::layout::Size> {
        self.0.size()
    }
    fn window_size(&mut self) -> io::Result<ratatui::backend::WindowSize> {
        self.0.window_size()
    }
    fn flush(&mut self) -> io::Result<()> {
        Backend::flush(&mut self.0)
    }
}

#[test]
fn resize_error_propagates_with_backend_message() {
    let root = tempfile::TempDir::new().expect("directory");
    let mut app = App::new().expect("app");
    app.left_panel = models::Panel::new(root.path().to_path_buf()).expect("panel");
    app.right_panel = models::Panel::new(root.path().to_path_buf()).expect("panel");
    let mut terminal = Terminal::new(FailClearBackend(WritableTestBackend::new(80, 24))).unwrap();
    let mut clicks = crate::mouse::PanelClicks::default();
    let err = handle_event(&mut terminal, &mut app, Event::Resize(60, 18), &mut clicks)
        .expect_err("resize must fail");
    assert!(err.to_string().contains("clear failed"));
}
