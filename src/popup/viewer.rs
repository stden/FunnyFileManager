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
    let page = Viewer::page_for(app.viewport);
    let command = resolve(&app.keymap.viewer, KeyEvent::new(code, modifiers));
    let Overlay::Viewer(viewer) = &mut app.overlay else {
        return false;
    };
    match command {
        Some(ViewerCommand::Close) => app.close_overlay(),
        Some(ViewerCommand::Scroll(scroll)) => scroll_body(viewer, scroll, page),
        None => {}
    }
    true
}

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
