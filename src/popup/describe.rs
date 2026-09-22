//! Purpose: Keys for the Describe files dialog (Ctrl+Z): type the text, Enter
//! writes it and moves to the next marked entry, Esc drops the rest.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::{App, DescribePrompt, Overlay};

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::Describe(prompt) = &mut app.overlay else {
        return false;
    };
    match code {
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Tab => prompt.toggle_button(),
        KeyCode::Enter if prompt.on_cancel() => app.close_overlay(),
        KeyCode::Enter => crate::ops::describe::accept(app),
        // On a button, Left and Right walk the buttons, as Far's do; in the
        // field they move the caret — see `edit`.
        KeyCode::Left | KeyCode::Right if prompt.on_cancel() => prompt.toggle_button(),
        KeyCode::Char(ch) if is_text(modifiers) => prompt.push(ch),
        _ => edit(prompt, code),
    }
    true
}

fn is_text(modifiers: KeyModifiers) -> bool {
    !modifiers.contains(KeyModifiers::CONTROL) && !modifiers.contains(KeyModifiers::ALT)
}

/// The field is an edit control: Left goes back to the typo, not to Cancel.
fn edit(prompt: &mut DescribePrompt, code: KeyCode) {
    match code {
        KeyCode::Left => prompt.left(),
        KeyCode::Right => prompt.right(),
        KeyCode::Home => prompt.home(),
        KeyCode::End => prompt.end(),
        KeyCode::Backspace => prompt.backspace(),
        KeyCode::Delete => prompt.delete_forward(),
        _ => {}
    }
}

#[cfg(test)]
mod tests;
