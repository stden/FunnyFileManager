//! Purpose: a context's bindings built from its table and the keys
//! `ffm.yml` gives — the four layers of the design's §6.4.
//!
//! 1. The keys the file gives an action are that action's.
//! 2. A chord two named actions claim goes to neither: a key that does
//!    nothing is safer than one that deletes.
//! 3. An action the file does not name has its defaults, less the chords
//!    the file gave to another action.
//! 4. An action without which the screen cannot be left never ends without
//!    a key a terminal delivers: its defaults come back, from whoever
//!    holds them.

use std::collections::{BTreeMap, BTreeSet};

use super::{ActionName, Row, Table};
use crate::action::{Bindings, KeyChord};

/// The keys the file gives: each action it names with its chords in the
/// file's order. An empty list leaves the action without a key; an action
/// that is absent keeps its defaults.
pub type Explicit = BTreeMap<ActionName, Vec<KeyChord>>;

/// What the layers had to decide, for the report to say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conflict {
    /// The file gives the chord to two actions; neither has it. `first`
    /// stands before `second` in the table.
    Claimed {
        chord: KeyChord,
        first: ActionName,
        second: ActionName,
    },
    /// `from` has the chord by default and the file gave it to `to`.
    DefaultDisplaced {
        chord: KeyChord,
        from: ActionName,
        to: ActionName,
    },
    /// The action is a way out of its screen and was left without a key
    /// every terminal delivers: its defaults are back.
    ExitRestored { action: ActionName },
}

/// A context's keys in force, and what was decided on the way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layered<C> {
    pub bindings: Bindings<C>,
    pub conflicts: Vec<Conflict>,
}

/// Who the file gives each chord to.
#[derive(Default)]
struct Claims {
    /// The first action to name each chord, in the table's order.
    first: BTreeMap<KeyChord, ActionName>,
    /// The chords another action names as well.
    contested: BTreeSet<KeyChord>,
}

impl Claims {
    /// Notes that the file gives `chord` to `action`; the conflict when
    /// another action claimed the chord first.
    fn claim(&mut self, chord: KeyChord, action: ActionName) -> Option<Conflict> {
        let first = *self.first.entry(chord).or_insert(action);
        if first == action {
            return None;
        }
        self.contested.insert(chord);
        Some(Conflict::Claimed {
            chord,
            first,
            second: action,
        })
    }
}

impl<C: Copy + PartialEq> Table<C> {
    /// The context's keys when no file names any: every action with its
    /// defaults.
    #[must_use]
    pub fn defaults(&self) -> Bindings<C> {
        self.layered(&Explicit::new()).bindings
    }

    /// The context's keys under the file's: see the module documentation.
    /// A name in `explicit` that is no action of this table is not read.
    #[must_use]
    pub fn layered(&self, explicit: &Explicit) -> Layered<C> {
        let mut layered = Layered {
            bindings: Bindings::default(),
            conflicts: Vec::new(),
        };
        let claims = self.claims(explicit, &mut layered.conflicts);
        for row in self.rows() {
            match explicit.get(&row.name) {
                Some(chords) => layered.bind_explicit(row, chords, &claims),
                None => layered.bind_defaults(row, &claims),
            }
        }
        layered.restore_exits(self);
        layered
    }

    /// Reads the file's keys in the table's order and reports a chord each
    /// time another action claims it as well. A chord an action lists
    /// twice is one claim.
    fn claims(&self, explicit: &Explicit, conflicts: &mut Vec<Conflict>) -> Claims {
        let mut claims = Claims::default();
        let named = self
            .rows()
            .filter_map(|row| explicit.get(&row.name).map(|chords| (row.name, chords)));
        for (action, chords) in named {
            let distinct: BTreeSet<KeyChord> = chords.iter().copied().collect();
            conflicts.extend(
                distinct
                    .into_iter()
                    .filter_map(|chord| claims.claim(chord, action)),
            );
        }
        claims
    }
}

impl<C: Copy + PartialEq> Layered<C> {
    fn bind_explicit(&mut self, row: &Row<C>, chords: &[KeyChord], claims: &Claims) {
        for &chord in chords {
            if !claims.contested.contains(&chord) {
                self.bindings.bind(row.command, chord);
            }
        }
    }

    fn bind_defaults(&mut self, row: &Row<C>, claims: &Claims) {
        for &chord in row.defaults {
            match claims.first.get(&chord) {
                Some(&to) => self.conflicts.push(Conflict::DefaultDisplaced {
                    chord,
                    from: row.name,
                    to,
                }),
                None => {
                    self.bindings.bind(row.command, chord);
                }
            }
        }
    }

    /// A restored action takes its defaults from whoever holds them, which
    /// may leave another way out bare: one pass for each.
    fn restore_exits(&mut self, table: &Table<C>) {
        for _ in table.exits() {
            let bare: Vec<&Row<C>> = table.exits().filter(|row| !self.leaves(row)).collect();
            for row in bare {
                self.restore(row);
            }
        }
    }

    /// Whether the action has a key that a terminal without the kitty
    /// protocol delivers as itself.
    fn leaves(&self, row: &Row<C>) -> bool {
        self.bindings
            .keys(row.command)
            .any(|chord| chord.delivery().on_legacy())
    }

    fn restore(&mut self, row: &Row<C>) {
        for &chord in row.defaults {
            self.bindings.bind(row.command, chord);
        }
        self.conflicts
            .push(Conflict::ExitRestored { action: row.name });
    }
}

#[cfg(test)]
mod tests;
