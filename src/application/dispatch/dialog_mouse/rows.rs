//! Purpose: a click on a dialog's body row — a field takes the cursor, a checkbox toggles, and a row that is only a sentence does nothing.

use crossterm::event::KeyEvent;

use crate::models::App;

mod more;

/// A click on a dialog's `index`-th body row.
///
/// Only the dialogs with controls in their body answer; elsewhere a row is a
/// sentence or a caption, and clicking a sentence should do nothing rather
/// than something the user cannot predict.
///
/// In both, a checkbox answers to the click that reached it and every other
/// control takes the cursor and waits — which is what clicking a text field
/// means everywhere else.
pub(super) fn click_dialog_row(app: &mut App, index: usize) -> Option<KeyEvent> {
    use crate::models::{EditorModal, Overlay};

    match &mut app.overlay {
        Overlay::Editor(editor) if matches!(editor.modal(), Some(EditorModal::Encoding(_))) => {
            if let Some(encoding) = crate::encoding::TextEncoding::ALL.get(index) {
                editor.encoding_choice = Some(*encoding);
            }
            None
        }
        Overlay::CopyPrompt(prompt) => {
            click_copy_row(prompt, index);
            None
        }
        Overlay::Filter(dialog) => {
            click_filter_row(dialog, &mut app.filter_input, index);
            None
        }
        Overlay::LspEditor(editor) => {
            click_lsp_form_row(editor, index);
            None
        }
        Overlay::NamePrompt(prompt) => {
            click_name_row(prompt, index);
            None
        }
        other => more::click_more_row(other, index),
    }
}

fn click_name_row(prompt: &mut crate::models::NamePrompt, index: usize) {
    if let Some(field) = prompt.field_at_row(index) {
        prompt.focus(field);
        if prompt.field() == crate::models::NameField::Multiple {
            prompt.toggle();
        }
    }
}

fn click_filter_row(
    dialog: &mut crate::models::FilterPrompt,
    input: &mut crate::models::FilterInput,
    index: usize,
) {
    let additional = input.additional.is_some();
    input.field = match index {
        0 => crate::models::FilterField::Include,
        1 if additional => crate::models::FilterField::Additional,
        1 | 2 if index == 1 || additional => crate::models::FilterField::Exclude,
        2 | 3 if index == 2 || additional => {
            dialog.hide_dotfiles = !dialog.hide_dotfiles;
            crate::models::FilterField::Hidden
        }
        _ => return,
    };
}

fn remote_row(dialog: &mut crate::models::remote_dialog::RemoteDialog, index: usize) {
    use crate::models::remote_dialog::{LoginField, RemoteDialog};
    match dialog {
        RemoteDialog::Login(login) if index == 2 => login.field = LoginField::User,
        RemoteDialog::Login(login) if index == 4 => login.field = LoginField::Password,
        RemoteDialog::Input { cancel, .. } if index == 1 => *cancel = false,
        _ => {}
    }
}

/// The Language servers form, like the copy dialog: a field takes the cursor
/// and Enabled toggles as well. The list, the view and the exit question have
/// no clickable rows, and neither has an existing server's dimmed language.
fn click_lsp_form_row(editor: &mut crate::models::LspEditor, index: usize) {
    use crate::models::lsp_editor::FormField;

    let (Some(form), Some(field)) = (editor.form_mut(), FormField::at_row(index)) else {
        return;
    };
    if !field.allows(form.target()) {
        return;
    }
    form.focus_on(field);
    if field == FormField::Enabled {
        form.toggle_enabled();
    }
}

fn click_copy_row(prompt: &mut crate::copy::CopyPrompt, index: usize) {
    let Some(field) = crate::copy::field_at_row(index) else {
        return;
    };
    if !prompt.allows(field) {
        return;
    }
    prompt.field = field;
    if field.is_checkbox() {
        prompt.toggle();
    }
}
