//! Name prompt keyboard controls, text editing, and acceptance.

use crate::models::{App, NameField, NamePrompt, Overlay};
use crossterm::event::{KeyCode, KeyModifiers};

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let directory = app.get_inactive_panel_ref().path.clone();
    let Overlay::NamePrompt(prompt) = &mut app.overlay else {
        return false;
    };
    match code {
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Tab | KeyCode::Down => prompt.advance(true),
        KeyCode::BackTab | KeyCode::Up => prompt.advance(false),
        KeyCode::Enter => {
            let cancelled = prompt.field() == NameField::Cancel;
            let settings = prompt.clone();
            app.close_overlay();
            // Nothing to do is the usual case, not an error: a blank
            // name, or a rename to the name it already has. Closing
            // quietly is the right answer to all of them.
            if !cancelled {
                crate::ops::name_prompt::apply(app, &settings);
            }
        }
        // A space is a space in a text field and a toggle everywhere
        // else, which is the only reading that leaves both usable.
        KeyCode::Char(' ') if !prompt.field().is_text() => prompt.toggle(),
        // The arrows belong to the control under the cursor: the caret in a
        // text field, the choice on the drop-down.
        KeyCode::Right if !prompt.field().is_text() => step_link(&directory, prompt, true),
        KeyCode::Left if !prompt.field().is_text() => step_link(&directory, prompt, false),
        _ => {
            if let Some(line) = prompt.focused_line_mut() {
                super::text_field::edit(line, code, modifiers, &mut app.clipboard);
            }
        }
    }
    true
}

/// Step the drop-down, passing over a kind the target cannot have.
///
/// Far's dialog lists a hard link for a file and not for a folder. The list
/// here is fixed per purpose, so the skip happens at the arrow instead: a
/// choice the arrows never stop on is a choice that is not offered.
fn step_link(directory: &std::path::Path, prompt: &mut NamePrompt, forward: bool) {
    for _ in 0..prompt.purpose().link_choices().len() {
        prompt.cycle(forward);
        if crate::ops::link::is_possible(directory, prompt) {
            return;
        }
    }
}

#[cfg(test)]
mod tests;
