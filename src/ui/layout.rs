//! Purpose: Screen layout constants, the panel page size they imply, and centred popup rectangles.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Screen layout (top to bottom): menu bar, panels (all remaining space),
/// command line, F-key bar. Named so [`panel_page_size`] stays in step with
/// the constraints used in [`render_panel_view`].
///
/// Far spends exactly two rows below the panels and one above them. The
/// banner and the status bar that used to sit there are gone: the banner said
/// only what the F-key bar already shows, and the status bar's one real job —
/// carrying messages — moved into the panel's own info row, which Far has and
/// this UI did not (see [`render_panel_info`]).
pub(super) const COMMAND_LINE_HEIGHT: u16 = 1;
pub(super) const FKEY_BAR_HEIGHT: u16 = 1;

/// Rows the menu bar takes at the top of the screen.
pub(super) const MENU_BAR_HEIGHT: u16 = 1;

/// Total non-panel lines claimed by the vertical layout above. The menu bar
/// counts: leaving it out made `PgDn` step one entry further than a panel
/// shows, so the first entry of every next page was never landed on.
const CHROME_HEIGHT: u16 = MENU_BAR_HEIGHT + COMMAND_LINE_HEIGHT + FKEY_BAR_HEIGHT;

/// Top and bottom border rows of a bordered widget (file panels, the
/// completion menu popup), which hold no content of their own.
pub(super) const PANEL_BORDERS: u16 = 2;

/// The column-title row under a panel's top border.
pub(super) const PANEL_COLUMN_HEADER: u16 = 1;

/// The rule that closes the column separators above the info row.
pub(super) const PANEL_COLUMN_RULE: u16 = 1;

/// The current-entry row above a panel's bottom border.
pub(super) const PANEL_INFO_ROW: u16 = 1;

/// Rows inside a panel's frame that are not entries.
pub(super) const PANEL_NON_ENTRY_ROWS: u16 =
    PANEL_COLUMN_HEADER + PANEL_COLUMN_RULE + PANEL_INFO_ROW;

/// Most candidate rows the completion dropdown shows at once.
pub(super) const MENU_MAX_ROWS: usize = 10;

/// Screen height assumed when the backend cannot report its size (a detached or
/// non-tty terminal). The classic 24-line screen keeps paging usable instead of
/// degrading to a one-row step.
pub const FALLBACK_TERMINAL_HEIGHT: u16 = 24;

/// Entry rows visible in one file panel — the distance PgUp/PgDn move.
///
/// Derived from `terminal_height` less the surrounding chrome. Never returns 0,
/// since a zero-size page would make paging silently do nothing on a very short
/// terminal.
#[must_use]
pub fn panel_page_size(terminal_height: u16) -> usize {
    let rows = terminal_height.saturating_sub(CHROME_HEIGHT + PANEL_BORDERS + PANEL_NON_ENTRY_ROWS);
    usize::from(rows).max(1)
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let percent_y = percent_y.min(100);
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    let percent_x = percent_x.min(100);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
