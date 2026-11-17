//! Purpose: cursor movement one row and one page at a time.

use super::*;

#[test]
fn test_navigation_moves_one_item() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    // Create multiple files to test navigation
    for i in 1..=5 {
        fs::write(temp_dir.path().join(format!("file{i}.txt")), b"data").expect("test setup");
    }

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Start at first item
    app.left_panel.select(Some(0));

    // Move down once
    app.left_panel.next();
    assert_eq!(
        app.left_panel.selected(),
        Some(1),
        "Should move to second item"
    );

    // Move down again
    app.left_panel.next();
    assert_eq!(
        app.left_panel.selected(),
        Some(2),
        "Should move to third item"
    );

    // Move up once
    app.left_panel.previous();
    assert_eq!(
        app.left_panel.selected(),
        Some(1),
        "Should move back to second item"
    );

    Ok(())
}

#[test]
fn test_apply_action_pages_selection_by_one_screen() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    for i in 0..40 {
        fs::write(temp_dir.path().join(format!("file{i:03}.txt")), b"x")?;
    }

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.active_panel = PanelSide::Left;
    app.left_panel.select(Some(0));

    // 80x24 terminal => 24 - 3 chrome (menu bar, command line, key bar)
    // - 2 borders - 3 in-panel rows (header, rule, info) = 16 visible rows,
    // so one page down lands on the entry just below the last visible one.
    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    apply_action(&mut terminal, &mut app, Action::PageDown, true);
    assert_eq!(app.left_panel.selected(), Some(16));

    apply_action(&mut terminal, &mut app, Action::PageUp, true);
    assert_eq!(app.left_panel.selected(), Some(0));

    Ok(())
}

struct CountingSizeBackend {
    inner: WritableTestBackend,
    size_queries: std::cell::Cell<usize>,
}

impl Write for CountingSizeBackend {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Write::flush(&mut self.inner)
    }
}

impl Backend for CountingSizeBackend {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a ratatui::buffer::Cell)>,
    {
        self.inner.draw(content)
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.inner.hide_cursor()
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.inner.show_cursor()
    }

    fn get_cursor_position(&mut self) -> io::Result<ratatui::layout::Position> {
        self.inner.get_cursor_position()
    }

    fn set_cursor_position<P: Into<ratatui::layout::Position>>(
        &mut self,
        position: P,
    ) -> io::Result<()> {
        self.inner.set_cursor_position(position)
    }

    fn clear(&mut self) -> io::Result<()> {
        self.inner.clear()
    }

    fn clear_region(&mut self, clear_type: ratatui::backend::ClearType) -> io::Result<()> {
        self.inner.clear_region(clear_type)
    }

    fn size(&self) -> io::Result<ratatui::layout::Size> {
        self.size_queries.set(self.size_queries.get() + 1);
        self.inner.size()
    }

    fn window_size(&mut self) -> io::Result<ratatui::backend::WindowSize> {
        self.inner.window_size()
    }

    fn flush(&mut self) -> io::Result<()> {
        Backend::flush(&mut self.inner)
    }
}

fn setup_brief_panel_and_console(temp_dir: &TempDir) -> io::Result<App> {
    for i in 0..40 {
        fs::write(temp_dir.path().join(format!("file{i:03}.txt")), b"x")?;
    }
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.left_panel.view = models::PanelView::Brief;
    app.active_panel = PanelSide::Left;
    app.left_panel.select(Some(0));
    for line in 0..60 {
        app.console.record_output(&format!("line {line}"));
    }
    Ok(app)
}

#[test]
fn page_and_column_navigation_do_not_query_backend_size() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = setup_brief_panel_and_console(&temp_dir)?;
    let mut terminal = Terminal::new(CountingSizeBackend {
        inner: WritableTestBackend::new(80, 24),
        size_queries: std::cell::Cell::new(0),
    })?;
    terminal.backend_mut().size_queries.set(0);
    for action in [
        Action::PageDown,
        Action::PageUp,
        Action::SelectPanel(PanelSide::Right),
        Action::SelectPanel(PanelSide::Left),
    ] {
        apply_action(&mut terminal, &mut app, action, true);
    }
    app.view = ViewMode::Console;
    apply_action(&mut terminal, &mut app, Action::PageDown, true);
    apply_action(&mut terminal, &mut app, Action::PageUp, true);
    assert_eq!(terminal.backend().size_queries.get(), 0);
    Ok(())
}
