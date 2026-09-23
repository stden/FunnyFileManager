//! Viewer navigation and dismissal.

use crate::models::{App, Overlay, Viewer};
use crossterm::event::{KeyCode, KeyModifiers};

const VIEWER_WIDE_STEP: usize = 20;

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let page = Viewer::page_for(app.viewport);
    let Overlay::Viewer(viewer) = &mut app.overlay else {
        return false;
    };
    // Far's viewer keys. The clamp lives on `Viewer`; this arm only
    // says which key means what, and how far.
    let ctrl = modifiers.contains(KeyModifiers::CONTROL);
    match code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::F(3 | 10) => {
            app.close_overlay();
        }
        KeyCode::Down => viewer.scroll_down(1, page),
        KeyCode::Up => viewer.scroll_up(1),
        KeyCode::PageDown => viewer.scroll_down(page, page),
        KeyCode::PageUp => viewer.scroll_up(page),
        KeyCode::Home => viewer.scroll_to_top(),
        KeyCode::End => viewer.scroll_to_bottom(page),
        KeyCode::Left if ctrl => viewer.scroll_left(VIEWER_WIDE_STEP),
        KeyCode::Right if ctrl => viewer.scroll_right(VIEWER_WIDE_STEP),
        KeyCode::Left => viewer.scroll_left(1),
        KeyCode::Right => viewer.scroll_right(1),
        _ => {}
    }
    true
}
