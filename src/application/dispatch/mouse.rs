//! Purpose: turn panel-area mouse presses and scrolls into the actions they stand for.

use std::io::Write;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::Backend, Terminal};

use crate::action::Action;
use crate::diagnostics::log_action;
use crate::models::{App, CommandLine, Overlay, ViewMode};

use super::{dialog_mouse, dispatch_action, dispatch_key, panel_mouse};

/// Turn a mouse click into the action the thing under it would have taken.
///
/// Only the left button, and only on press: a release is the same gesture
/// arriving a second time, and acting on both would run every command twice.
///
/// A gesture is translated into the *action* it stands for — the wheel is
/// `cursor_up`, a double click is `open` — and dispatched the way that
/// action's key is, rather than into an effect of its own. That is what keeps
/// the mouse from becoming a second implementation of the program: a double
/// click runs exactly what Enter runs, including whatever guards and dialogs
/// Enter goes through. A key-bar cell is the one gesture that is a key: it
/// presses `F(n)` with the modifiers held, which is what its label says.
pub(in crate::application) fn dispatch_mouse<B>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    mouse: crossterm::event::MouseEvent,
    area: ratatui::layout::Rect,
    panel_clicks: &mut crate::mouse::PanelClicks,
) where
    B: Backend + Write,
{
    let target = panel_mouse::hit(app, area, mouse.column, mouse.row);
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

/// The wheel is a cursor move: over the console it scrolls the transcript,
/// over a panel it moves that panel's cursor — the tree's, the preview's or
/// the sessions list's when the panel is one of those.
fn dispatch_scroll<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    mouse: crossterm::event::MouseEvent,
    target: crate::mouse::Target,
    panel_clicks: &mut crate::mouse::PanelClicks,
) -> bool {
    use crate::mouse::{Target, Wheel};
    let Some(wheel) = Wheel::of(mouse.kind) else {
        return false;
    };
    let action = match wheel {
        Wheel::Up => Action::NavUp,
        Wheel::Down => Action::NavDown,
    };
    panel_clicks.reset();
    if app.view == ViewMode::Console {
        dispatch_action(terminal, app, action);
    } else if let Target::PanelRow { side, .. } | Target::Panel(side) = target {
        app.active_panel = side;
        dispatch_action(terminal, app, action);
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
        Target::PanelRow { side, row } => {
            panel_mouse::click_row(terminal, app, side, row, panel_clicks);
        }
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
