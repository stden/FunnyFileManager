//! Purpose: The panel screen: the menu bar, the two panels — or one, when the
//! other is hidden or a view takes the whole width — the command line with
//! its completion menu, and the key bar.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use super::command_line::render_command_line;
use super::completion::render_completion_menu;
use super::fkey_bar::{panel_key_labels, render_key_bar};
use super::layout::{COMMAND_LINE_HEIGHT, FKEY_BAR_HEIGHT, MENU_BAR_HEIGHT};
use super::panel::{panel_footer, render_panel};
use super::{menu, quickview, remote, sessions, tree};
use crate::models::{App, PanelKind, PanelSide, PanelsShown};

pub(super) fn render_panel_view(f: &mut Frame<'_>, app: &mut App, area: Rect) {
    // No transcript layer here. It used to be painted underneath and "covered
    // by the panels", but covered was never true: a ratatui widget sets
    // symbols only where its text reaches, so the menu bar's 44 cells of
    // headings left the tail of every longer transcript line on screen — and
    // repainted it from the log on every frame, so it never went away. The
    // transcript belongs to the console screen, which `console::render_view`
    // still paints; `render_backdrop` owns the ground here.
    let [menu_bar, panels, command_line, fkey_bar] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(MENU_BAR_HEIGHT),
            Constraint::Min(0),
            Constraint::Length(COMMAND_LINE_HEIGHT),
            Constraint::Length(FKEY_BAR_HEIGHT),
        ])
        .areas(area);
    app.dialog_mouse.screen = area;
    menu::render_bar(f, app, menu_bar);
    render_both_panels(f, app, panels);
    render_command_line(f, app, command_line);
    render_completion_menu(f, app, command_line);
    render_key_bar(f, fkey_bar, &panel_key_labels(fkey_bar.width));
}

fn render_both_panels(f: &mut Frame<'_>, app: &mut App, area: Rect) {
    // Only the active panel replaces its selected-entry footer with status.
    let footer = panel_footer(app);
    let (left, right) = panel_areas(app, area);
    if let Some(area) = left {
        render_one_panel(f, app, PanelSide::Left, area, footer.as_ref());
    }
    if let Some(area) = right {
        render_one_panel(f, app, PanelSide::Right, area, footer.as_ref());
    }
}

/// Where each panel goes, or `None` for one that is off screen.
///
/// A hidden panel (`Ctrl+F1` / `Ctrl+F2`) gives its half to the other, and a
/// fullscreen view mode on the active panel takes the whole width — Far's
/// Detailed columns do not fit in half a screen, which is why Far marks them
/// fullscreen in the first place.
pub fn panel_areas(app: &App, area: Rect) -> (Option<Rect>, Option<Rect>) {
    // `mouse::hit` reads the same answer, so a click lands where this drew.
    match app.panels_on_screen() {
        PanelsShown::LeftOnly => (Some(area), None),
        PanelsShown::RightOnly => (None, Some(area)),
        PanelsShown::Both => {
            let [left, right] = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(area);
            (Some(left), Some(right))
        }
    }
}

/// One panel, drawn as whatever kind it is: a file list, a folder tree, a
/// preview of the other panel's cursor, or a `NetBox` list. The file and
/// `NetBox` panels give their info row to the status when they are active.
fn render_one_panel(
    f: &mut Frame<'_>,
    app: &mut App,
    side: PanelSide,
    area: Rect,
    footer: Option<&crate::status::PathMessage<'_>>,
) {
    let is_active = app.active_panel == side;
    let footer = footer.filter(|_| is_active);
    let panel = app.panel_mut(side);
    match panel.kind {
        PanelKind::Files => render_panel(f, panel, area, is_active, footer),
        PanelKind::Tree(_) => tree::render(f, panel, area, is_active, footer),
        PanelKind::QuickView(_) => quickview::render(f, panel, area, is_active),
        PanelKind::Sessions(_) => sessions::render(f, panel, area, is_active, footer),
        PanelKind::Remote(_) => remote::render(f, panel, area, is_active, footer),
    }
}
