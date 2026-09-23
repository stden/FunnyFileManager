//! Purpose: Dialog geometry recorded by the renderer, and what a click inside a dialog lands on.

use ratatui::layout::Rect;

/// Where the dialog on screen was drawn, and how far it has been dragged.
///
/// Unlike the panels, a dialog's geometry is not a function of the screen size
/// alone: it is sized to its own content, which only the renderer assembles.
/// So the renderer records what it drew, and the next click is read against
/// that. There is no staleness to guard against — the loop draws immediately
/// before it reads an event, so the record always describes the frame the user
/// was looking at when they clicked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DialogMouse {
    /// The whole dialog, or `None` when none is drawn.
    pub area: Option<Rect>,
    /// The body's rows: where they start, and how many there are.
    pub body: Option<Rect>,
    /// Each button's index and the columns it covers, left inclusive, right
    /// exclusive.
    pub buttons: Vec<(usize, u16, u16)>,
    /// The row the buttons sit on.
    pub buttons_y: u16,
    /// How far the user has dragged the dialog from centre.
    pub offset: (i32, i32),
    /// Where the pointer took hold, while a drag is in progress.
    pub grabbed: Option<(u16, u16)>,
    /// The screen the last frame was drawn on. The menu's geometry hangs off
    /// the top row, so a click on it needs the same rectangle the renderer had.
    pub screen: Rect,
}

/// What a click inside a dialog landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogTarget {
    /// A button, by its position in the dialog's button list.
    Button(usize),
    /// A body row, by its offset from the top of the body.
    Row(usize),
    /// The dialog, but nothing in it — its frame, margin or a blank line.
    /// This is what a drag takes hold of.
    Chrome,
    /// Outside the dialog entirely.
    Outside,
}

impl DialogMouse {
    /// Forget the last frame's geometry, keeping where the dialog was dragged
    /// to. Called when no dialog is drawn.
    pub fn clear(&mut self) {
        self.area = None;
        self.body = None;
        self.buttons.clear();
    }

    /// Forget everything, including the drag. A newly opened dialog opens
    /// centred rather than wherever the last one was pushed to.
    pub fn reset(&mut self) {
        self.clear();
        self.offset = (0, 0);
        self.grabbed = None;
    }

    /// What is at `(column, row)`.
    #[must_use]
    pub fn hit(&self, column: u16, row: u16) -> DialogTarget {
        let Some(area) = self.area else {
            return DialogTarget::Outside;
        };
        if column < area.x || column >= area.right() || row < area.y || row >= area.bottom() {
            return DialogTarget::Outside;
        }
        if row == self.buttons_y {
            return self
                .buttons
                .iter()
                .find_map(|&(index, left, right)| {
                    (left..right)
                        .contains(&column)
                        .then_some(DialogTarget::Button(index))
                })
                .unwrap_or(DialogTarget::Chrome);
        }
        let Some(body) = self.body else {
            return DialogTarget::Chrome;
        };
        // The last two body rows are the rule and buttons, not actionable rows.
        let rows = body.height.saturating_sub(2);
        let offset = row.checked_sub(body.y).filter(|offset| *offset < rows);
        if (body.x..body.x + body.width).contains(&column) {
            return offset.map_or(DialogTarget::Chrome, |offset| {
                DialogTarget::Row(usize::from(offset))
            });
        }
        DialogTarget::Chrome
    }
}
