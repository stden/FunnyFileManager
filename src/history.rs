//! Purpose: Persistent `:` command history with a type-gated durability rule.
//!
//! Everything executed is remembered for the session; only lines the
//! [`DurableLine`] gate accepts are written to disk. The gate consults
//! [`Command::parse`](crate::command::Command::parse): `Shell` and `Invalid`
//! lines — the ones that can carry passwords and tokens — cannot even be
//! *represented* as durable, so no code path can write them. See the secrets
//! rule in `keepass.rs` and the design spec.

use std::collections::VecDeque;
use std::fs;

mod durable;
mod storage;

pub use durable::DurableLine;
use storage::history_file_path;
pub use storage::Persistence;
pub(crate) use storage::{data_file, write_owner_only};

/// Most lines the on-disk history keeps; oldest fall off first.
pub const HISTORY_CAP: usize = 500;

/// Defensive upper bound on the history file we will read into memory. We only
/// ever write `HISTORY_CAP` (500) lines of paths — far under a mebibyte — so a
/// file larger than this is hand-made or corrupt; rather than slurp it whole
/// with `read_to_string`, we treat it as empty (completion must never wedge the
/// file manager over a bad history file).
const MAX_HISTORY_FILE_BYTES: u64 = 1 << 20;

/// A position in the history ring, counted back from the newest entry.
///
/// A newtype because `History::recall` and `History::search_back` both take
/// "how far back", and so does `ConsoleLog`'s scroll — three `usize`s measuring
/// different rings that a plain integer lets you hand to the wrong one. The
/// step methods keep the walk in one place too, so `Up` and `Down` cannot drift
/// apart in how they move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct RingIndex(usize);

impl RingIndex {
    /// The most recent entry.
    pub const NEWEST: Self = Self(0);

    #[must_use]
    pub const fn new(offset: usize) -> Self {
        Self(offset)
    }

    /// One step further back in time.
    #[must_use]
    pub const fn older(self) -> Self {
        Self(self.0 + 1)
    }

    /// One step towards the present, or `None` at the newest entry — which is
    /// what tells `Down` to restore the line the user was typing.
    #[must_use]
    pub const fn newer(self) -> Option<Self> {
        match self.0.checked_sub(1) {
            Some(offset) => Some(Self(offset)),
            None => None,
        }
    }

    #[must_use]
    pub const fn offset(self) -> usize {
        self.0
    }
}

/// Session-plus-disk command history.
///
/// `session` holds everything executed this run, newest last. `durable`
/// mirrors the on-disk file (newest last), re-read before every save so that
/// several running instances add to one history instead of overwriting each
/// other. Suggestions search the session ring first — most recent experience
/// wins — then the persisted lines.
///
/// `persistence` is the write target, resolved once by [`History::load`] and
/// never re-read: [`Persistence::SessionOnly`] makes [`History::save`]
/// permanently a no-op for this instance — the target lives in the type, so no
/// call site can accidentally write to disk. `default()` is always
/// session-only; it is what `App::new()` starts with (see `models.rs`), so
/// merely constructing an `App` never touches disk. Only production
/// `main_impl` (`lib.rs`) replaces it with `History::load()`.
///
/// `Clone` is safe today only because nothing clones a `History` that holds
/// a `Persistence::File`: the sole whole-`App` clone in the tree is a `#[cfg(test)]`
/// helper (`src/tests_quick_search_nav.rs`) exercising a session-only
/// history. Two live clones sharing one file merge like two processes do:
/// each save re-reads the file first. Only saves that interleave between
/// that read and the atomic rename can still lose a line — there is no lock.
#[derive(Debug, Default, Clone)]
pub struct History {
    session: VecDeque<String>,
    durable: VecDeque<DurableLine>,
    persistence: Persistence,
}

impl History {
    /// Read the persisted history and remember where it came from. Missing,
    /// unreadable, or corrupt files are an empty history — completion must
    /// never break the file manager. The path is resolved exactly once,
    /// here, so a later `save()` cannot race a concurrent environment change
    /// and write somewhere other than where this load actually read from.
    #[must_use]
    pub fn load() -> Self {
        let persistence = history_file_path().map_or(Persistence::SessionOnly, Persistence::File);
        let durable = persistence.file().map(read_durable).unwrap_or_default();
        Self {
            session: VecDeque::new(),
            durable,
            persistence,
        }
    }

    /// Remember an executed line. Always kept for the session; appended to
    /// disk only when [`DurableLine::classify`] accepts it AND this history
    /// has a persistence target (see `path` on the struct).
    pub fn record(&mut self, line: &str) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return;
        }
        push_deduped(&mut self.session, trimmed.to_owned(), |a, b| a == b);

        if let Some(durable) = DurableLine::classify(trimmed) {
            // Another ffm may have saved since this one loaded. Starting from
            // the file makes the save add to its lines; starting from memory
            // wrote back a stale copy and dropped them.
            if let Some(path) = self.persistence.file() {
                self.durable = read_durable(path);
            }
            push_deduped(&mut self.durable, durable, PartialEq::eq);
            self.save();
        }
    }

    /// Rewrite the history file from `self.durable`, owner-only where the OS
    /// supports it. A no-op when `self.path` is `None` (session-only
    /// history — the common case in tests, and the default `App::new()`
    /// starts with). All failures are swallowed: losing history is
    /// acceptable, breaking the file manager is not.
    fn save(&self) {
        let Some(path) = self.persistence.file() else {
            return;
        };
        if let Some(dir) = path.parent() {
            if fs::create_dir_all(dir).is_err() {
                return;
            }
        }
        let mut text = String::new();
        for line in &self.durable {
            text.push_str(line.as_str());
            text.push('\n');
        }
        let _ = write_owner_only(path, &text);
    }

    /// The most recent entry having `prefix` as a literal prefix and more
    /// text after it. Literal, never fuzzy: `Right` accepts the result
    /// without confirmation, so it must be exactly what it looks like.
    #[must_use]
    pub fn suggest(&self, prefix: &str) -> Option<&str> {
        if prefix.is_empty() {
            return None;
        }
        self.newest_first()
            .find(|entry| entry.starts_with(prefix) && entry.len() > prefix.len())
    }

    /// Every remembered line, newest first: the session ring, then the
    /// persisted lines it does not already contain.
    ///
    /// The filter matters for `Up`: a durable line executed this run lives in
    /// *both* rings, and walking back through the same command twice reads as
    /// a stuck key. `save` writes only from `durable`, so filtering the read
    /// side cannot affect what reaches disk.
    fn newest_first(&self) -> impl Iterator<Item = &str> {
        self.session.iter().rev().map(String::as_str).chain(
            self.durable
                .iter()
                .rev()
                .map(DurableLine::as_str)
                .filter(|line| !self.session.iter().any(|seen| seen == line)),
        )
    }

    /// The `index`-th line counting back from the newest (`0` is the most
    /// recent), or `None` past the oldest — which is what stops `Up` at the
    /// end of the ring instead of wrapping around to the newest again.
    #[must_use]
    pub fn recall(&self, index: RingIndex) -> Option<&str> {
        self.newest_first().nth(index.offset())
    }

    /// Index of the first line at or after `from` (counting back from the
    /// newest) that *contains* `query` — the `Ctrl+R` lookup.
    ///
    /// Substring rather than fuzzy, and deliberately unlike [`Self::suggest`]:
    /// reverse search shows its hit in the command line before `Enter` accepts
    /// it, so a looser match costs the user nothing but a second `Ctrl+R`.
    /// An empty query matches nothing, so `Ctrl+R` alone does not silently
    /// recall the last command.
    #[must_use]
    pub fn search_back(&self, query: &str, from: RingIndex) -> Option<RingIndex> {
        if query.is_empty() {
            return None;
        }
        self.newest_first()
            .enumerate()
            .skip(from.offset())
            .find(|(_, entry)| entry.contains(query))
            .map(|(index, _)| RingIndex::new(index))
    }
}

/// The durable lines of the history file at `path`. Missing, unreadable,
/// oversize or corrupt files are an empty history — completion must never
/// break the file manager.
fn read_durable(path: &std::path::Path) -> VecDeque<DurableLine> {
    // Skip the read entirely for an oversize (hand-made / corrupt) file.
    if !path
        .metadata()
        .is_ok_and(|meta| meta.len() <= MAX_HISTORY_FILE_BYTES)
    {
        return VecDeque::new();
    }
    fs::read_to_string(path)
        .map(|text| {
            text.lines()
                // classify() re-checks on load: a hand-edited file cannot
                // smuggle a shell line back into the durable set.
                .filter_map(DurableLine::classify)
                .collect()
        })
        .unwrap_or_default()
}

/// Push `value`, dropping any older equal entry (dedup keep-newest) and the
/// oldest lines beyond [`HISTORY_CAP`].
fn push_deduped<T>(ring: &mut VecDeque<T>, value: T, eq: impl Fn(&T, &T) -> bool) {
    ring.retain(|existing| !eq(existing, &value));
    ring.push_back(value);
    while ring.len() > HISTORY_CAP {
        ring.pop_front();
    }
}

#[cfg(test)]
mod tests;
