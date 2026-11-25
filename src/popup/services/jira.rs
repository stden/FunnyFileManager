//! Purpose: Keyboard handling inside the Jira browser, detail reader, and JQL editor.

use crossterm::event::{KeyCode, KeyModifiers};

use super::{edit_active_line, motion_for, plain_modifiers, scroll_lines};
use crate::lineedit::LineEditor;
use crate::models::services::jira::JiraMode;
use crate::models::{App, Overlay};
use crate::ops::jira::{actions, edit_config};

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    let Overlay::Jira(dialog) = &mut app.overlay else {
        return;
    };
    let is_jql = match &mut dialog.mode {
        JiraMode::Detail { .. } => {
            detail_key(&mut dialog.mode, code);
            return;
        }
        JiraMode::EditJql(_) if code == KeyCode::Esc => {
            dialog.mode = JiraMode::Browse;
            return;
        }
        JiraMode::EditJql(_) => true,
        JiraMode::Browse => false,
    };
    if is_jql {
        process_jql(app, code, modifiers);
    } else {
        process_browse(app, code, modifiers);
    }
}

fn detail_key(mode: &mut JiraMode, code: KeyCode) {
    if matches!(code, KeyCode::Esc | KeyCode::Enter | KeyCode::F(3)) {
        *mode = JiraMode::Browse;
        return;
    }
    if let (Some(motion), JiraMode::Detail { lines, scroll, .. }) = (motion_for(code), mode) {
        scroll_lines(scroll, lines.len(), motion);
    }
}

fn process_jql(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    if code == KeyCode::Enter {
        actions::submit_jql(app);
    } else {
        edit_active_line(app, code, modifiers, pick_jql);
    }
}

pub(super) fn pick_jql(overlay: &mut Overlay) -> Option<&mut LineEditor> {
    let Overlay::Jira(dialog) = overlay else {
        return None;
    };
    match &mut dialog.mode {
        JiraMode::EditJql(editor) => Some(editor),
        _ => None,
    }
}

fn process_browse(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    let ctrl = modifiers.contains(KeyModifiers::CONTROL);
    let shift = modifiers.contains(KeyModifiers::SHIFT);
    match code {
        KeyCode::Esc => escape_browse(app),
        KeyCode::Enter | KeyCode::F(3) => actions::view_selected(app),
        KeyCode::F(2) => actions::begin_edit_jql(app),
        KeyCode::Char('j' | 'J') if ctrl => actions::begin_edit_jql(app),
        KeyCode::F(4) => drop(edit_config(app)),
        KeyCode::F(5) => actions::queue_export_selected(app, shift),
        KeyCode::F(6) => actions::queue_export_all(app, shift),
        KeyCode::F(7) => actions::queue_snapshot(app),
        KeyCode::Char('s' | 'S') if ctrl => actions::queue_snapshot(app),
        KeyCode::Char('r' | 'R') if ctrl => actions::reload(app),
        _ => browse_nav_or_filter(app, code, modifiers),
    }
}

pub(super) fn browse_nav_or_filter(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    let Overlay::Jira(dialog) = &mut app.overlay else {
        return;
    };
    match code {
        KeyCode::Char(' ') | KeyCode::Insert => dialog.toggle_mark(),
        KeyCode::Backspace => dialog.erase_filter(),
        KeyCode::Char(ch) if plain_modifiers(modifiers) => dialog.type_filter(ch),
        _ => {
            if let Some(motion) = motion_for(code) {
                dialog.move_cursor(motion);
            }
        }
    }
}

fn escape_browse(app: &mut App) {
    let clear_only = match &mut app.overlay {
        Overlay::Jira(dialog) if !dialog.filter.is_empty() => {
            dialog.filter.clear();
            dialog.cursor = 0;
            true
        }
        _ => false,
    };
    if !clear_only {
        app.close_overlay();
    }
}
