//! Purpose: Password input has no history, copy operation or unmasked presentation.

use crate::models::remote_dialog::{LoginField, RemoteLogin};
use crate::models::ClipboardSlot;
use crossterm::event::{KeyCode, KeyModifiers};

pub(super) fn key(
    login: &mut RemoteLogin,
    code: KeyCode,
    modifiers: KeyModifiers,
    clipboard: &mut ClipboardSlot,
) {
    match code {
        KeyCode::Tab | KeyCode::Down => login.field = login.field.next(),
        KeyCode::BackTab | KeyCode::Up => login.field = login.field.next().next().next(),
        _ if login.field == LoginField::User => {
            super::super::text_field::edit(&mut login.user, code, modifiers, clipboard);
        }
        _ if login.field == LoginField::Password => {
            password_key(&mut login.credentials.password, code, modifiers, clipboard);
        }
        _ => {}
    }
}

fn password_key(
    password: &mut zeroize::Zeroizing<String>,
    code: KeyCode,
    modifiers: KeyModifiers,
    clipboard: &mut ClipboardSlot,
) {
    let ctrl = modifiers.contains(KeyModifiers::CONTROL);
    if (code == KeyCode::Char('v') && ctrl)
        || (code == KeyCode::Insert && modifiers == KeyModifiers::SHIFT)
    {
        if let Some(Ok(text)) = clipboard
            .backend_mut()
            .map(crate::clipboard::ClipboardBackend::get_native)
        {
            let text = zeroize::Zeroizing::new(text);
            append_password(password, &text);
        }
        return;
    }
    match code {
        KeyCode::Backspace => {
            password.pop();
        }
        KeyCode::Char('u' | 'y') if ctrl => password.clear(),
        KeyCode::Char(ch)
            if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                && password.len() < 4096 =>
        {
            password.push(ch);
        }
        _ => {}
    }
}

fn append_password(password: &mut zeroize::Zeroizing<String>, text: &str) {
    let remaining = 4096usize.saturating_sub(password.len());
    password.extend(text.chars().filter(|ch| !ch.is_control()).take(remaining));
}

pub(super) fn paste(login: &mut RemoteLogin, text: &str) {
    match login.field {
        LoginField::Password => append_password(&mut login.credentials.password, text),
        LoginField::User => login
            .user
            .insert_str(&super::super::text_field::one_line(text)),
        LoginField::Connect | LoginField::Cancel => {}
    }
}
