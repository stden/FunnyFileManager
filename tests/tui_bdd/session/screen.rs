//! Purpose: Reading the current frame: text, panels, size, clipboard and how the session ended.

use ratatui::style::Color;

use super::*;

/// The editor paints its selection in the UI palette's `CURSOR_BG`. Pinned by
/// value on purpose: the harness checks the colour a user sees, so a palette
/// change must also be an edit here.
const SELECTION_BG: Color = Color::Rgb(45, 110, 175);

impl Session {
    #[expect(
        clippy::panic,
        reason = "the BDD harness: a scenario reading a screen after the TUI exited fails here"
    )]
    pub(super) fn buffer(&self) -> &Buffer {
        match &self.current {
            Output::Frame(buffer) => buffer,
            Output::Exited(result) => panic!("expected a frame, got {result:?}"),
        }
    }

    pub(in crate::tui_bdd) fn size(&self) -> Size {
        self.buffer().area.as_size()
    }

    pub(in crate::tui_bdd) fn text(&self) -> String {
        buffer_text(self.buffer(), 0..self.size().width)
    }

    /// The text painted in the selection colour, one line per row that has any.
    pub(in crate::tui_bdd) fn highlighted(&self) -> String {
        let buffer = self.buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .filter(|&x| buffer[(x, y)].bg == SELECTION_BG)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .filter(|row| !row.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[expect(
        clippy::expect_used,
        reason = "the BDD harness: a TUI that stops answering or panics fails the scenario driving it"
    )]
    pub(in crate::tui_bdd) fn clipboard(&self) -> String {
        self.clipboard.lock().expect("clipboard").clone()
    }

    #[expect(
        clippy::panic,
        reason = "the BDD harness: a scenario naming no panel side is a test bug and fails here"
    )]
    pub(in crate::tui_bdd) fn panel_text(&self, side: &str) -> String {
        let width = self.size().width;
        let columns = match side {
            "left" => 0..width / 2,
            "right" => width / 2..width,
            _ => panic!("unknown panel {side}"),
        };
        buffer_text(self.buffer(), columns)
    }

    pub(in crate::tui_bdd) fn assert_exited(&self) {
        assert!(
            matches!(self.current, Output::Exited(Ok(()))),
            "session must exit normally"
        );
    }
}
