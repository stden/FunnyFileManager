//! Purpose: turn panel-area mouse presses and scrolls into the keys they stand for.

use std::io::Write;
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::Backend, Terminal};

use crate::diagnostics::log_action;
use crate::models::{self, App, CommandLine, Overlay, ViewMode};

use super::{dialog_mouse, dispatch_key, panel_mouse};

/// Turn a mouse click into the action the thing under it would have taken.
///
/// Only the left button, and only on press: a release is the same gesture
/// arriving a second time, and acting on both would run every command twice.
///
/// A click is translated into the *key* it stands for rather than into an
/// effect of its own. That is what keeps the mouse from becoming a second
/// implementation of the program: clicking `5Copy` runs exactly what F5 runs,
/// including whatever guards and dialogs F5 goes through.
pub(in crate::application) fn dispatch_mouse<B>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    mouse: crossterm::event::MouseEvent,
    area: ratatui::layout::Rect,
    panel_clicks: &mut crate::mouse::PanelClicks,
) where
    B: Backend + Write,
{
    let target = crate::mouse::hit(
        area,
        mouse.column,
        mouse.row,
        crate::ui::FKEY_COUNT,
        app.panels_on_screen(),
    );
    let target = panel_mouse::target(app, target, area, mouse.column);
    if overlay_consumes_mouse(terminal, app, mouse, target, panel_clicks)
        || dispatch_scroll(terminal, app, mouse, target, panel_clicks)
        || !is_panel_press(mouse, panel_clicks)
    {
        return;
    }
    if mouse.row == area.y {
        if let Some(heading) = crate::ui::heading_at(area, mouse.column) {
            panel_clicks.reset();
            app.set_menu(crate::menu::Menu::open(heading));
            return;
        }
    }
    log_action(&format!("Mouse click: {target:?}"));
    dispatch_target(terminal, app, target, mouse.modifiers, panel_clicks);
}

fn overlay_consumes_mouse<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    mouse: crossterm::event::MouseEvent,
    target: crate::mouse::Target,
    panel_clicks: &mut crate::mouse::PanelClicks,
) -> bool {
    use crate::mouse::Target;
    use crossterm::event::{MouseButton, MouseEventKind};
    if !app.overlay.is_active() {
        return false;
    }
    panel_clicks.reset();
    let returns_to_panels = app.view == ViewMode::Panels
        && matches!(&app.overlay, Overlay::CommandLine(_))
        && mouse.kind == MouseEventKind::Down(MouseButton::Left)
        && matches!(target, Target::PanelRow { .. } | Target::Panel(_));
    if returns_to_panels {
        app.close_overlay();
        false
    } else {
        dialog_mouse::dispatch_dialog_mouse(terminal, app, mouse);
        true
    }
}

fn dispatch_scroll<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    mouse: crossterm::event::MouseEvent,
    target: crate::mouse::Target,
    panel_clicks: &mut crate::mouse::PanelClicks,
) -> bool {
    use crate::mouse::Target;
    use crossterm::event::MouseEventKind;
    let code = match mouse.kind {
        MouseEventKind::ScrollUp => KeyCode::Up,
        MouseEventKind::ScrollDown => KeyCode::Down,
        _ => return false,
    };
    panel_clicks.reset();
    if app.view == ViewMode::Console {
        dispatch_key(terminal, app, KeyEvent::new(code, mouse.modifiers));
    } else if let Target::PanelRow { side, .. } | Target::Panel(side) = target {
        app.active_panel = side;
        dispatch_key(terminal, app, KeyEvent::new(code, mouse.modifiers));
    }
    true
}

fn is_panel_press(
    mouse: crossterm::event::MouseEvent,
    clicks: &mut crate::mouse::PanelClicks,
) -> bool {
    use crossterm::event::{MouseButton, MouseEventKind};
    if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
        return true;
    }
    if mouse.kind != MouseEventKind::Up(MouseButton::Left) {
        clicks.reset();
    }
    false
}

fn dispatch_target<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    target: crate::mouse::Target,
    modifiers: KeyModifiers,
    panel_clicks: &mut crate::mouse::PanelClicks,
) {
    use crate::mouse::Target;
    match target {
        Target::FunctionKey(number) => {
            panel_clicks.reset();
            dispatch_key(terminal, app, KeyEvent::new(KeyCode::F(number), modifiers));
        }
        Target::PanelRow { side, row } => click_panel_row(terminal, app, side, row, panel_clicks),
        Target::Panel(side) => {
            panel_clicks.reset();
            app.active_panel = side;
        }
        Target::CommandLine => {
            panel_clicks.reset();
            if !matches!(app.overlay, Overlay::CommandLine(_)) {
                app.set_overlay(Overlay::CommandLine(CommandLine::default()));
            }
        }
        Target::Nothing => panel_clicks.reset(),
    }
}

fn click_panel_row<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    side: models::PanelSide,
    row: usize,
    panel_clicks: &mut crate::mouse::PanelClicks,
) {
    app.active_panel = side;
    if app.get_active_panel_ref().kind.is_netbox() {
        click_netbox_row(terminal, app, side, row, panel_clicks);
        return;
    }
    let panel = app.get_active_panel();
    let index = panel.list_state_mut().offset() + row;
    if index >= panel.entries.len() {
        panel_clicks.reset();
        return;
    }
    let click = panel_clicks.press(side, index, Instant::now());
    panel.select(Some(index));
    if click == crate::mouse::PanelRowClick::Activate {
        dispatch_key(
            terminal,
            app,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );
    }
}

/// A click on a row of a `NetBox` panel: the panel's own cursor moves there —
/// never the hidden file list's — and a double click is Enter on it.
fn click_netbox_row<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    side: models::PanelSide,
    row: usize,
    panel_clicks: &mut crate::mouse::PanelClicks,
) {
    let Some(index) = app.get_active_panel().kind.select_visible_row(row) else {
        panel_clicks.reset();
        return;
    };
    if panel_clicks.press(side, index, Instant::now()) == crate::mouse::PanelRowClick::Activate {
        dispatch_key(
            terminal,
            app,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );
    }
}
