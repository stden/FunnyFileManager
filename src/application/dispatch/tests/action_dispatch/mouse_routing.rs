//! Mouse dispatch through panels, modal input, menu controls and the event loop.

use super::*;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

// ─── the mouse ──────────────────────────────────────────────────────────────

fn mouse_at(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    }
}

fn newest(app: &App) -> String {
    app.console
        .visible(10)
        .last()
        .map(|line| line.text().to_owned())
        .unwrap_or_default()
}

fn panel_row_at(area: ratatui::layout::Rect, column: u16, accept: impl Fn(usize) -> bool) -> u16 {
    (area.y..area.bottom())
        .find(|row| {
            matches!(
                crate::mouse::hit(area, column, *row, crate::ui::FKEY_COUNT, models::PanelsShown::Both),
                crate::mouse::Target::PanelRow {
                    side: PanelSide::Left,
                    row
                } if accept(row)
            )
        })
        .expect("project row is on screen")
}

fn click_at(
    terminal: &mut Terminal<WritableTestBackend>,
    app: &mut App,
    position: (u16, u16),
    clicks: &mut crate::mouse::PanelClicks,
) {
    let area = terminal.get_frame().area();
    dispatch_mouse(
        terminal,
        app,
        mouse_at(
            MouseEventKind::Down(MouseButton::Left),
            position.0,
            position.1,
        ),
        area,
        clicks,
    );
}

#[cfg(test)]
mod tests_bars;
#[cfg(test)]
mod tests_panel_focus;
#[cfg(test)]
mod tests_row_clicks;
#[cfg(test)]
mod tests_wheel;
