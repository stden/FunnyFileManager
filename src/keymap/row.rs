//! Purpose: a key table — the rows of one context, each the single place
//! that says what an action is called, which command it runs, its default
//! keys, where the key comes from and what the action does.

use std::fmt;

use super::Context;
use crate::action::KeyChord;

/// An action's name in `ffm.yml`: `snake_case`, unique in its context, and
/// stable. The name exists at the file's boundary only; dispatch compares
/// commands, never names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionName(&'static str);

impl ActionName {
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for ActionName {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(self.0)
    }
}

/// Where an action's default key comes from: the text the default file
/// writes after the summary (`Far: Gray +`, `htop: P`, `FFM`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Far Manager's key for the function is the first default.
    FarSame,
    /// Far Manager's key for the function, where the first default is
    /// another or Far spells it differently: `F4` beside Enter, `Gray +`
    /// for `+`.
    Far(&'static str),
    /// A Far Manager command that Far gives no key.
    FarNoKey,
    /// htop's key for the function.
    Htop(&'static str),
    /// bash's (readline's) key for the function.
    Bash(&'static str),
    /// This program's own.
    Ffm,
}

/// What an action is to its screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// A command like any other: the file may leave it without a key.
    Command,
    /// A way out of the screen, which never loses its last key.
    Exit,
}

/// One action of one context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row<C: 'static> {
    pub name: ActionName,
    /// The one command the name stands for.
    pub command: C,
    /// The keys the action has when the file does not name it, the one a
    /// menu prints first.
    pub defaults: &'static [KeyChord],
    pub origin: Origin,
    /// What the action does: the file's comment, the help page, the report.
    pub summary: &'static str,
    pub role: Role,
}

/// A row as the tables start one: the action's name, its command and its
/// default keys. [`Row::note`] completes it.
#[must_use]
pub const fn row<C: Copy>(name: &'static str, command: C, defaults: &'static [KeyChord]) -> Row<C> {
    Row {
        name: ActionName::new(name),
        command,
        defaults,
        origin: Origin::Ffm,
        summary: "",
        role: Role::Command,
    }
}

impl<C: Copy> Row<C> {
    /// The row with where its key comes from and what the action does.
    #[must_use]
    pub const fn note(self, origin: Origin, summary: &'static str) -> Self {
        Self {
            origin,
            summary,
            ..self
        }
    }

    /// The row as a way out of its screen ([`Role::Exit`]).
    #[must_use]
    pub const fn exit(self) -> Self {
        Self {
            role: Role::Exit,
            ..self
        }
    }
}

/// Rows that belong together, under the heading the default file gives them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Group<C: 'static> {
    pub title: &'static str,
    pub rows: &'static [Row<C>],
}

/// Every action of one context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Table<C: 'static> {
    pub context: Context,
    pub groups: &'static [Group<C>],
}

impl<C> Table<C> {
    /// The rows of every group, in the order the default file writes them.
    pub fn rows(&self) -> impl Iterator<Item = &'static Row<C>> {
        self.groups.iter().flat_map(|group| group.rows)
    }

    /// The rows without which the screen cannot be left.
    pub fn exits(&self) -> impl Iterator<Item = &'static Row<C>> {
        self.rows().filter(|row| row.role == Role::Exit)
    }
}
