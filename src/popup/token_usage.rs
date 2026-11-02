//! Purpose: Usage report navigation, manual refresh and modal close.
use crate::action::viewer::{resolve, ViewerCommand};
use crate::models::token_usage::UsageState;
use crate::models::{App, Overlay, Viewer};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let command = resolve(&app.keymap.viewer, KeyEvent::new(code, modifiers));
    let page = Viewer::page_for(app.viewport);
    let Overlay::TokenUsage(panel) = &mut app.overlay else {
        return false;
    };
    if code == KeyCode::F(5) && modifiers.is_empty() {
        panel.generation = std::sync::Arc::new(());
        panel.state = UsageState::Queued;
    } else {
        match command {
            Some(ViewerCommand::Close) => app.close_overlay(),
            Some(ViewerCommand::Scroll(scroll)) => {
                super::viewer::scroll_body(&mut panel.viewer, scroll, page);
            }
            None => {}
        }
    }
    true
}
