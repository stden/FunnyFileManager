//! Purpose: Keys for a panel that has become a folder tree (`Ctrl+T`).
//!
//! The dispatcher asks here first while the active panel is a tree; `true`
//! means the key was the tree's. The tree claims what moves, opens, searches
//! and closes its rows and nothing more, so `Ctrl+T` still turns it off,
//! `Tab` still switches panels and `:` still opens the command line — the
//! rows are folders, not entries, and every key that would act on entries
//! either goes to the file panel or is told to wait for one.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::models::{App, FolderTree};
use crate::ops::tree::{self, tree_mut};
use crate::status::Status;

/// The one line the tree gives F5–F8: they need a file panel to act on.
const NEEDS_FILE_PANEL: &str = "Tree panel: press Enter to open the folder, then F5–F8 work there";

/// Whether the tree panel consumed `key`.
pub fn process(app: &mut App, key: KeyEvent) -> bool {
    if tree_mut(app).is_none() {
        return false;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return control_key(app, key.code);
    }
    match key.code {
        KeyCode::Enter => {
            tree::enter(app);
            true
        }
        KeyCode::Esc => clear_search(app),
        KeyCode::Backspace => shorten_search(app),
        KeyCode::F(5..=8) => {
            app.set_status(Status::CommandOutput(NEEDS_FILE_PANEL.to_owned()));
            true
        }
        KeyCode::Char(':') => false,
        // Far's `Gray +` / `Gray -`; terminals cannot tell the pad's keys
        // from the row's, so the plain characters serve, as they do for
        // marking on a file panel.
        KeyCode::Char('+') => moved(app, FolderTree::next_sibling),
        KeyCode::Char('-') => moved(app, FolderTree::previous_sibling),
        KeyCode::Char(ch) => search(app, ch),
        code => cursor_key(app, code),
    }
}

fn control_key(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Enter => moved(app, |tree| {
            let query = tree.search.query().unwrap_or_default().to_owned();
            tree.find_next(&query)
        }),
        KeyCode::Char('r' | 'R') => {
            tree::refresh(app);
            true
        }
        _ => false,
    }
}

/// The keys that only move the cursor. Every one is the tree's even when
/// the cursor is already at the end it is being pushed towards: a file panel
/// would not do anything else with them either.
fn cursor_key(app: &mut App, code: KeyCode) -> bool {
    let page = crate::ui::panel_page_size(app.viewport.rows());
    match code {
        KeyCode::Right => {
            tree::expand_or_descend(app);
            true
        }
        KeyCode::Left => moved(app, |tree| tree.collapse() || tree.to_parent()),
        KeyCode::Up
        | KeyCode::Down
        | KeyCode::Home
        | KeyCode::End
        | KeyCode::PageUp
        | KeyCode::PageDown => moved(app, |tree| {
            step(tree, code, page);
            true
        }),
        _ => false,
    }
}

fn step(tree: &mut FolderTree, code: KeyCode, page: usize) {
    match code {
        KeyCode::Up => tree.up(),
        KeyCode::Down => tree.down(),
        KeyCode::Home => tree.first(),
        KeyCode::End => tree.last(),
        KeyCode::PageUp => tree.page_up(page),
        KeyCode::PageDown => tree.page_down(page),
        _ => {}
    }
}

/// Run a cursor move and, when it moved, let the other panel follow. Always
/// `true`: the key was the tree's whether or not there was room to move.
fn moved(app: &mut App, step: impl FnOnce(&mut FolderTree) -> bool) -> bool {
    if tree_mut(app).is_some_and(step) {
        tree::follow_cursor(app);
    }
    true
}

/// Typed letters find a folder by name — Far's fast find, without having to
/// hold Alt, because a tree row is nothing a typed command could run. When
/// the longer query matches nothing the search restarts from the new letter,
/// as the file panel's does.
fn search(app: &mut App, ch: char) -> bool {
    moved(app, |tree| {
        let query = tree.search.push(ch).to_owned();
        if tree.find_first(&query) {
            return true;
        }
        if !tree.search.should_restart() {
            return false;
        }
        let restarted = tree.search.restart_with(ch).to_owned();
        tree.find_first(&restarted)
    })
}

fn shorten_search(app: &mut App) -> bool {
    let Some(tree) = tree_mut(app) else {
        return false;
    };
    if !tree.search.is_active() {
        return false;
    }
    if let Some(query) = tree.search.backspace().map(str::to_owned) {
        tree.find_first(&query);
        tree::follow_cursor(app);
    }
    true
}

fn clear_search(app: &mut App) -> bool {
    let Some(tree) = tree_mut(app) else {
        return false;
    };
    if !tree.search.is_active() {
        return false;
    }
    tree.search.clear();
    true
}

#[cfg(test)]
mod tests;
