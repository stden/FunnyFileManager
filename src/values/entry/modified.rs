//! Purpose: An entry's timestamp as the panel's Date and Time columns show it,
//! degrading to blank when the filesystem will not say.

use std::cell::RefCell;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{Datelike, Timelike};

#[derive(Clone, Copy)]
struct LocalParts {
    month: u8,
    day: u8,
    year: u8,
    hour: u8,
    minute: u8,
}

thread_local! {
    static MINUTE_CACHE: RefCell<[Option<(u64, LocalParts)>; 256]> =
        const { RefCell::new([None; 256]) };
}

/// One of an entry's timestamps — modified, created or accessed — as the
/// panel's Date and Time columns show it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Modified(Option<SystemTime>);

impl Modified {
    /// No readable timestamp: both columns render empty.
    pub const UNKNOWN: Self = Self(None);

    #[must_use]
    pub const fn new(at: SystemTime) -> Self {
        Self(Some(at))
    }

    /// Read an entry's mtime, degrading to [`Self::UNKNOWN`] rather than
    /// failing the whole listing.
    #[must_use]
    pub fn from_metadata(metadata: &std::fs::Metadata) -> Self {
        Self::from_result(metadata.modified())
    }

    /// The same degradation for `created()` and `accessed()`.
    #[must_use]
    pub fn from_result(at: std::io::Result<SystemTime>) -> Self {
        at.map_or(Self::UNKNOWN, Self::new)
    }

    fn parts(self) -> Option<LocalParts> {
        let at = self.0?;
        let Ok(since_epoch) = at.duration_since(UNIX_EPOCH) else {
            return Some(resolve_parts(at));
        };
        let minute_key = since_epoch.as_secs() / 60;
        let slot_idx = usize::try_from(minute_key & 255).unwrap_or(0);
        if let Some(hit) = MINUTE_CACHE.with(|cache| {
            cache
                .borrow()
                .get(slot_idx)
                .and_then(|slot| *slot)
                .and_then(|(k, parts)| (k == minute_key).then_some(parts))
        }) {
            return Some(hit);
        }
        let resolved = resolve_parts(at);
        MINUTE_CACHE.with(|cache| {
            if let Some(slot) = cache.borrow_mut().get_mut(slot_idx) {
                *slot = Some((minute_key, resolved));
            }
        });
        Some(resolved)
    }

    /// `MM/DD/YY`, or empty when unknown.
    #[must_use]
    pub fn date(self) -> String {
        let Some(parts) = self.parts() else {
            return String::new();
        };
        let mut out = String::with_capacity(8);
        push_date(&mut out, parts);
        out
    }

    /// `HH:MM` in 24-hour form, or empty when unknown.
    #[must_use]
    pub fn time(self) -> String {
        let Some(parts) = self.parts() else {
            return String::new();
        };
        let mut out = String::with_capacity(5);
        push_time(&mut out, parts);
        out
    }

    /// `MM/DD/YY HH:MM`, or empty when unknown.
    #[must_use]
    pub(crate) fn stamp(self) -> String {
        let Some(parts) = self.parts() else {
            return String::new();
        };
        let mut out = String::with_capacity(14);
        push_date(&mut out, parts);
        out.push(' ');
        push_time(&mut out, parts);
        out
    }
}

fn resolve_parts(at: SystemTime) -> LocalParts {
    let dt = chrono::DateTime::<chrono::Local>::from(at);
    LocalParts {
        month: u8::try_from(dt.month()).unwrap_or(0),
        day: u8::try_from(dt.day()).unwrap_or(0),
        year: u8::try_from(dt.year().rem_euclid(100)).unwrap_or(0),
        hour: u8::try_from(dt.hour()).unwrap_or(0),
        minute: u8::try_from(dt.minute()).unwrap_or(0),
    }
}

fn push_two(out: &mut String, value: u8) {
    out.push(char::from(b'0' + (value / 10) % 10));
    out.push(char::from(b'0' + value % 10));
}

fn push_date(out: &mut String, parts: LocalParts) {
    push_two(out, parts.month);
    out.push('/');
    push_two(out, parts.day);
    out.push('/');
    push_two(out, parts.year);
}

fn push_time(out: &mut String, parts: LocalParts) {
    push_two(out, parts.hour);
    out.push(':');
    push_two(out, parts.minute);
}

#[cfg(test)]
mod tests;
