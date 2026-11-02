//! Purpose: Mouse events for the harness, and where a text sits on the rendered screen.

use super::*;

// ─── the mouse ──────────────────────────────────────────────────────────────

/// A click on the key bar, at the column the label was drawn in.
pub fn click(column: u16, row: u16) -> Event {
    click_with(column, row, KeyModifiers::empty())
}

pub fn click_with(column: u16, row: u16, modifiers: KeyModifiers) -> Event {
    Event::Mouse(crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column,
        row,
        modifiers,
    })
}

pub fn scroll_down(column: u16, row: u16) -> Event {
    Event::Mouse(crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::ScrollDown,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    })
}

// ─── the mouse inside dialogs ───────────────────────────────────────────────

/// Where `needle` sits on the rendered screen, as (column, row).
///
/// Dialogs are centred and sized to their content, so their buttons have no
/// fixed address. Finding the text is how a test clicks what a user would.
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test assertion: text missing from the screen fails the calling test; clippy.toml \
              allows that in #[test] fns and #[cfg(test)] items, not in integration-test helpers. \
              The u16 conversions cannot fail: the 100x24 test screen is far smaller than u16::MAX"
)]
pub fn find(screen: &str, needle: &str) -> (u16, u16) {
    // Columns, not byte offsets. The frames are box-drawing characters at
    // three bytes each, so `str::find` answers a different question than the
    // one a mouse event asks.
    let wanted: Vec<char> = needle.chars().collect();
    for (row, line) in screen.lines().enumerate() {
        let columns: Vec<char> = line.chars().collect();
        if let Some(column) = columns
            .windows(wanted.len())
            .position(|window| window == wanted.as_slice())
        {
            return (
                u16::try_from(column).expect("column fits"),
                u16::try_from(row).expect("row fits"),
            );
        }
    }
    panic!("{needle:?} is not on screen:\n{screen}");
}

pub fn drag(column: u16, row: u16) -> Event {
    Event::Mouse(crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Drag(crossterm::event::MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::empty(),
    })
}
