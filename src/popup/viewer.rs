//! Purpose: The viewer overlay's keys: Far's viewer commands, read from the
//! keymap's `viewer` bindings by `action::viewer` and applied to the open
//! `Viewer` here. The quick view's preview scrolls through the same
//! `scroll_body`.
//!
//! Every key is consumed: the viewer is modal, so a key it does not use
//! never reaches the panels behind it.

use crate::action::viewer::{resolve, ViewerCommand, ViewerScroll};
use crate::models::{App, Overlay, Viewer};
use crate::mouse::Wheel;
use crate::scroll::Scrolled;
use crate::scroll::WIDE_STEP;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let command = resolve(&app.keymap.viewer, KeyEvent::new(code, modifiers));
    let Overlay::Viewer(viewer) = &mut app.overlay else {
        return false;
    };
    if viewer.image.is_some()
        && !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
        && image_key(viewer, code)
    {
        return true;
    }
    if let Some(command) = command {
        apply(app, command);
    }
    true
}

/// Execute a viewer command independently of its input binding.
pub fn apply(app: &mut App, command: ViewerCommand) {
    let page = Viewer::page_for(app.viewport);
    let Overlay::Viewer(viewer) = &mut app.overlay else {
        return;
    };
    match command {
        ViewerCommand::Close => app.close_overlay(),
        ViewerCommand::Scroll(scroll) => scroll_body(viewer, scroll, page),
    }
}

fn image_key(viewer: &mut Viewer, code: KeyCode) -> bool {
    match code {
        KeyCode::Char('+' | '=') => viewer.image_zoom = viewer.image_zoom.saturating_add(1).min(8),
        KeyCode::Char('-') => viewer.image_zoom = viewer.image_zoom.saturating_sub(1).max(1),
        KeyCode::Char('0') => {
            viewer.image_zoom = 1;
            viewer.image_pan = (0, 0);
        }
        KeyCode::Left => viewer.image_pan.0 = viewer.image_pan.0.saturating_sub(64),
        KeyCode::Right => viewer.image_pan.0 = viewer.image_pan.0.saturating_add(64),
        KeyCode::Up => viewer.image_pan.1 = viewer.image_pan.1.saturating_sub(64),
        KeyCode::Down => viewer.image_pan.1 = viewer.image_pan.1.saturating_add(64),
        _ => return false,
    }
    clamp_image_pan(viewer);
    true
}

fn clamp_image_pan(viewer: &mut Viewer) {
    let Some(crate::models::ImageState::Ready(image)) = &viewer.image else {
        viewer.image_pan = (0, 0);
        return;
    };
    let zoom = u32::from(viewer.image_zoom.max(1));
    let max_x = image.width().saturating_sub((image.width() / zoom).max(1));
    let max_y = image
        .height()
        .saturating_sub((image.height() / zoom).max(1));
    viewer.image_pan.0 = viewer.image_pan.0.min(max_x);
    viewer.image_pan.1 = viewer.image_pan.1.min(max_y);
}

#[cfg(test)]
mod tests;

/// The wheel: a line up or down, as the arrows are. `false` when no viewer
/// is open.
pub(super) fn wheel(app: &mut App, wheel: Wheel) -> bool {
    let page = Viewer::page_for(app.viewport);
    let Overlay::Viewer(viewer) = &mut app.overlay else {
        return false;
    };
    let scroll = match wheel {
        Wheel::Up => ViewerScroll::LineUp,
        Wheel::Down => ViewerScroll::LineDown,
    };
    scroll_body(viewer, scroll, page);
    true
}

/// One scroll command on the body the viewer or the quick view shows. The
/// clamp lives on [`Scrolled`]; this only says how far each command moves.
pub(super) fn scroll_body(body: &mut impl Scrolled, scroll: ViewerScroll, page: usize) {
    match scroll {
        ViewerScroll::LineUp => body.scroll_up(1),
        ViewerScroll::LineDown => body.scroll_down(1, page),
        ViewerScroll::PageUp => body.scroll_up(page),
        ViewerScroll::PageDown => body.scroll_down(page, page),
        ViewerScroll::Top => body.scroll_to_top(),
        ViewerScroll::Bottom => body.scroll_to_bottom(page),
        ViewerScroll::Left => body.scroll_left(1),
        ViewerScroll::Right => body.scroll_right(1),
        ViewerScroll::LeftWide => body.scroll_left(WIDE_STEP),
        ViewerScroll::RightWide => body.scroll_right(WIDE_STEP),
    }
}
