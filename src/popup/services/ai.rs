//! Purpose: Keyboard handling inside the Warp-like AI assistant chat overlay.

use crossterm::event::{KeyCode, KeyModifiers};

use super::edit_active_line;
use crate::lineedit::LineEditor;
use crate::models::{App, Overlay};
use crate::ops::ai::{actions, edit_config};
use crate::values::ai::AiProvider;

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    let ctrl = modifiers.contains(KeyModifiers::CONTROL);
    let alt = modifiers.contains(KeyModifiers::ALT);
    match code {
        KeyCode::Esc => app.close_overlay(),
        KeyCode::Enter => actions::submit_prompt(app),
        KeyCode::Tab => actions::cycle_provider(app),
        KeyCode::Char('p' | 'P') if ctrl => actions::cycle_provider(app),
        KeyCode::Char('1') if alt => actions::select_provider(app, AiProvider::Anthropic),
        KeyCode::Char('2') if alt => actions::select_provider(app, AiProvider::OpenAi),
        KeyCode::Char('3') if alt => actions::select_provider(app, AiProvider::Codex),
        KeyCode::F(2) => drop(actions::save_transcript(app)),
        KeyCode::Char('s' | 'S') if ctrl => drop(actions::save_transcript(app)),
        KeyCode::F(4) => drop(edit_config(app)),
        KeyCode::F(5) => actions::insert_command(app),
        KeyCode::Char('x' | 'X') if ctrl && is_prompt_empty(app) => actions::insert_command(app),
        KeyCode::F(8) => actions::clear_history(app),
        KeyCode::Char('l' | 'L') if ctrl => actions::clear_history(app),
        KeyCode::Up => actions::scroll_history(app, -1),
        KeyCode::Down => actions::scroll_history(app, 1),
        KeyCode::PageUp => actions::scroll_history(app, -6),
        KeyCode::PageDown => actions::scroll_history(app, 6),
        _ => edit_active_line(app, code, modifiers, pick_prompt),
    }
}

fn is_prompt_empty(app: &App) -> bool {
    matches!(&app.overlay, Overlay::Ai(dialog) if dialog.prompt.is_empty())
}

fn pick_prompt(overlay: &mut Overlay) -> Option<&mut LineEditor> {
    match overlay {
        Overlay::Ai(dialog) => Some(&mut dialog.prompt),
        _ => None,
    }
}
