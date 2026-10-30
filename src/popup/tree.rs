//! Purpose: Keys for a panel that has become a folder tree (`Ctrl+T`).
//!
//! The dispatcher asks here first while the active panel is a tree; `true`
//! means the key was the tree's. The tree claims what moves, opens, searches
//! and closes its rows and nothing more, so `Ctrl+T` still turns it off,
//! `Tab` still switches panels and `:` still opens the command line — the
//! rows are folders, not entries, and every key that would act on entries
//! either goes to the file panel or is told to wait for one. Which key means
//! what is `action::tree`'s; this file applies the command.

use crossterm::event::KeyEvent;

use crate::action::tree::{resolve, TreeCommand};
use crate::models::{App, FolderTree, ListMotion};
use crate::ops::tree::{self, tree_mut};
use crate::status::Status;

/// The one line the tree gives F5–F8: they need a file panel to act on.
const NEEDS_FILE_PANEL: &str = "Tree panel: press Enter to open the folder, then F5–F8 work there";

/// Whether the tree panel consumed `key`.
pub fn process(app: &mut App, key: KeyEvent) -> bool {
    if tree_mut(app).is_none() {
        return false;
    }
    resolve(key).is_some_and(|command| apply(app, command))
}

/// Apply `command` to the tree; `false` when the tree had no use for it
/// after all (a search key with no search running), so the key falls
/// through to the file panel.
pub fn apply(app: &mut App, command: TreeCommand) -> bool {
    match command {
        TreeCommand::Move(motion) => move_cursor(app, motion),
        TreeCommand::Expand => expand(app),
        TreeCommand::Collapse => moved(app, |tree| tree.collapse() || tree.move_to_parent()),
        TreeCommand::GoToFolder => go_to_folder(app),
        TreeCommand::NextSibling => moved(app, FolderTree::next_sibling),
        TreeCommand::PreviousSibling => moved(app, FolderTree::previous_sibling),
        TreeCommand::FindNext => find_next(app),
        TreeCommand::RefreshTree => refresh(app),
        TreeCommand::ClearSearch => clear_search(app),
        TreeCommand::ShortenSearch => shorten_search(app),
        TreeCommand::Search(ch) => search(app, ch),
        TreeCommand::NeedsFilePanel => needs_file_panel(app),
    }
}

/// A cursor move is the tree's even when the cursor is already at the end
/// it is being pushed towards: a file panel would not do anything else with
/// the key either.
fn move_cursor(app: &mut App, motion: ListMotion) -> bool {
    let page = crate::ui::panel_page_size(app.viewport.rows());
    moved(app, |tree| {
        step(tree, motion, page);
        true
    })
}

fn expand(app: &mut App) -> bool {
    tree::expand_or_descend(app);
    true
}

fn go_to_folder(app: &mut App) -> bool {
    tree::enter(app);
    true
}

fn find_next(app: &mut App) -> bool {
    moved(app, |tree| {
        let query = tree.search.query().unwrap_or_default().to_owned();
        tree.find_next(&query)
    })
}

fn refresh(app: &mut App) -> bool {
    tree::refresh(app);
    true
}

fn needs_file_panel(app: &mut App) -> bool {
    app.set_status(Status::CommandOutput(NEEDS_FILE_PANEL.to_owned()));
    true
}

fn step(tree: &mut FolderTree, motion: ListMotion, page: usize) {
    match motion {
        ListMotion::Up => tree.up(),
        ListMotion::Down => tree.down(),
        ListMotion::Home => tree.first(),
        ListMotion::End => tree.last(),
        ListMotion::PageUp => tree.page_up(page),
        ListMotion::PageDown => tree.page_down(page),
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

/// `false` without a search to shorten: the key is then the file panel's.
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

/// `false` without a search to end: the key is then the file panel's.
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
