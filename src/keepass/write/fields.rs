//! Purpose: the five standard fields an entry is written with, through either
//! of the `keepass` crate's entry writers.

use keepass::db::{fields, EntryMut, EntryTrack};

use crate::keepass::EntryDetails;

/// The entry setters shared by [`EntryMut`] and [`EntryTrack`], so the list of
/// standard fields in [`write_entry_fields`] lives in exactly one place.
pub(in crate::keepass) trait EntryFieldWriter {
    fn set_unprotected_field(&mut self, key: &str, value: &str);
    fn set_protected_field(&mut self, key: &str, value: &str);
}

impl EntryFieldWriter for EntryMut<'_> {
    fn set_unprotected_field(&mut self, key: &str, value: &str) {
        self.set_unprotected(key, value);
    }

    fn set_protected_field(&mut self, key: &str, value: &str) {
        self.set_protected(key, value);
    }
}

impl EntryFieldWriter for EntryTrack<'_> {
    fn set_unprotected_field(&mut self, key: &str, value: &str) {
        self.set_unprotected(key, value);
    }

    fn set_protected_field(&mut self, key: &str, value: &str) {
        self.set_protected(key, value);
    }
}

/// Write the five standard fields onto an entry (password is stored protected).
pub(in crate::keepass) fn write_entry_fields(
    entry: &mut impl EntryFieldWriter,
    details: &EntryDetails,
) {
    entry.set_unprotected_field(fields::TITLE, details.title.as_str());
    entry.set_unprotected_field(fields::USERNAME, details.username.as_str());
    entry.set_unprotected_field(fields::URL, details.url.as_str());
    entry.set_unprotected_field(fields::NOTES, details.notes.as_str());
    entry.set_protected_field(fields::PASSWORD, details.password.as_str());
}
