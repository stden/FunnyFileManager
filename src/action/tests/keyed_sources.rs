//! Purpose: every action that reaches a panel kind without a key, beside the key it stands for.
//!
//! `from_action` stays until step 6 of the keymap design (spec §14.5), so
//! until then each panel kind reads an action in one table and a key in
//! another. These are the key-less sources of an action — a menu row that
//! prints a shortcut, the wheel's two cursor moves and the double click's
//! Open — so a test can hold the two tables to one meaning.

use crossterm::event::KeyEvent;

use crate::action::{Action, KeyChord};
use crate::menu::{Entry, Heading, Menu};

/// An action that arrives without a key, the key it stands for, and the
/// shortcut text that names it when the two disagree.
pub(in crate::action) struct KeyedSource {
    pub action: Action,
    pub key: KeyEvent,
    pub shortcut: &'static str,
}

/// Every key-less way an action reaches a panel kind.
pub(in crate::action) fn keyed_sources() -> Vec<KeyedSource> {
    let rows = Heading::ALL
        .into_iter()
        .flat_map(|heading| Menu::open(heading).entries().iter().copied())
        .filter_map(row_source);
    let mouse = [
        (Action::NavUp, "Up"),
        (Action::NavDown, "Down"),
        (Action::Open, "Enter"),
    ]
    .into_iter()
    .map(|(action, shortcut)| source(action, shortcut));
    rows.chain(mouse).collect()
}

/// A row with a printed shortcut.
fn row_source(entry: Entry) -> Option<KeyedSource> {
    let Entry::Item {
        shortcut, action, ..
    } = entry
    else {
        return None;
    };
    if shortcut.is_empty() {
        return None;
    }
    Some(source(action, shortcut))
}

/// Far's "Gray +" is the plain `+` here, since terminals cannot tell the
/// pad's keys from the row's.
fn source(action: Action, shortcut: &'static str) -> KeyedSource {
    let text = shortcut.strip_prefix("Gray ").unwrap_or(shortcut);
    let chord: KeyChord = text
        .parse()
        .unwrap_or_else(|error| panic!("{shortcut:?}: {error}"));
    KeyedSource {
        action,
        key: chord.to_event(),
        shortcut,
    }
}
