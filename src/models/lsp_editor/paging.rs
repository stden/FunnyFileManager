//! Purpose: Paging the server list and scrolling the F3 view by what the last
//! frame drew.

use crate::scroll::ScrollTop;

use super::{LspEditor, Mode};

/// A move of the list selection or the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Up,
    Down,
    PageUp,
    PageDown,
    First,
    Last,
}

/// What the last frame drew, which paging needs. Recorded by the renderer,
/// like the dialog mouse; a frame draws the list screen or the F3 view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drawn {
    /// The list screen, its server list `page` rows tall.
    List { page: usize },
    /// The F3 view: `lines` in all, `page` of them on screen.
    View { lines: usize, page: usize },
}

impl Default for Drawn {
    fn default() -> Self {
        Self::List { page: 1 }
    }
}

impl Drawn {
    /// How far a list page moves the selection: at least one row.
    pub(super) fn list_page(self) -> usize {
        match self {
            Self::List { page } => page.max(1),
            Self::View { .. } => 1,
        }
    }

    /// How far a view page scrolls: at least one line.
    fn view_page(self) -> usize {
        match self {
            Self::View { page, .. } => page.max(1),
            Self::List { .. } => 1,
        }
    }

    /// The last line the view can start at, with its last line at the bottom.
    fn last_view_top(self) -> usize {
        match self {
            Self::View { lines, page } => lines.saturating_sub(page),
            Self::List { .. } => 0,
        }
    }
}

impl LspEditor {
    /// Record what the frame just drew, and keep the view in range.
    pub fn record_drawn(&mut self, drawn: Drawn) {
        self.drawn = drawn;
        let last = drawn.last_view_top();
        if let Mode::View(top) = &mut self.mode {
            *top = ScrollTop::TOP.down(top.lines(), last);
        }
    }

    /// Scroll the F3 view within what the last frame drew.
    pub fn scroll_view(&mut self, step: Step) {
        let last = self.drawn.last_view_top();
        let page = self.drawn.view_page();
        let Mode::View(top) = &mut self.mode else {
            return;
        };
        *top = match step {
            Step::Up => top.up(1),
            Step::Down => top.down(1, last),
            Step::PageUp => top.up(page),
            Step::PageDown => top.down(page, last),
            Step::First => ScrollTop::TOP,
            Step::Last => ScrollTop::TOP.down(last, last),
        };
    }
}
