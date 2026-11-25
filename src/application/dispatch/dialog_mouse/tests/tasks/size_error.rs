//! Purpose: Task list mouse clicks bail cleanly when terminal size cannot be queried.

use super::*;

struct UnsizedBackend(WritableTestBackend, bool);

impl Write for UnsizedBackend {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Write::flush(&mut self.0)
    }
}

impl Backend for UnsizedBackend {
    type Error = std::io::Error;

    fn draw<'a, I>(&mut self, content: I) -> std::io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a ratatui::buffer::Cell)>,
    {
        self.0.draw(content)
    }

    fn hide_cursor(&mut self) -> std::io::Result<()> {
        self.0.hide_cursor()
    }

    fn show_cursor(&mut self) -> std::io::Result<()> {
        self.0.show_cursor()
    }

    fn get_cursor_position(&mut self) -> std::io::Result<ratatui::layout::Position> {
        self.0.get_cursor_position()
    }

    fn set_cursor_position<P: Into<ratatui::layout::Position>>(
        &mut self,
        position: P,
    ) -> std::io::Result<()> {
        self.0.set_cursor_position(position)
    }

    fn clear(&mut self) -> std::io::Result<()> {
        self.0.clear()
    }

    fn clear_region(&mut self, clear_type: ratatui::backend::ClearType) -> std::io::Result<()> {
        self.0.clear_region(clear_type)
    }

    fn size(&self) -> std::io::Result<ratatui::layout::Size> {
        if self.1 {
            Err(std::io::Error::other("size unavailable"))
        } else {
            self.0.size()
        }
    }

    fn window_size(&mut self) -> std::io::Result<ratatui::backend::WindowSize> {
        self.0.window_size()
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Backend::flush(&mut self.0)
    }
}

#[test]
fn task_list_click_returns_false_when_terminal_size_fails() {
    let mut term = Terminal::new(UnsizedBackend(WritableTestBackend::new(80, 24), false)).unwrap();
    term.backend_mut().1 = true;
    let mut tasks = three();
    assert!(!super::super::super::tasks::click_task_list(
        &mut term,
        &mut tasks.app,
        5,
        5,
    ));
}
