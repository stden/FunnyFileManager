//! Purpose: The rules the Windows PATH store applies to the registry's `Path`
//! value — what a missing one reads as, what a save compares, which kind a
//! new value is stored as — kept apart from the registry calls so every
//! platform tests them.

use crate::platform::{RegistryString, RegistryStringKind};

/// `Path` as the registry holds it, or as a missing one reads: empty and
/// expandable, the kind Windows itself gives it.
pub(super) fn stored(value: Option<RegistryString>) -> RegistryString {
    value.unwrap_or(RegistryString {
        kind: RegistryStringKind::Expandable,
        text: String::new(),
    })
}

/// What a save compares to learn whether `Path` changed since it was read:
/// its kind and its text.
pub(super) fn revision(value: &RegistryString) -> String {
    format!("{}|{}", value.kind.label(), value.text)
}

/// `text` as it replaces `current`: expandable when `current` was, or when
/// `text` names a `%VARIABLE%`, so the reference keeps expanding; plain
/// otherwise.
pub(super) fn replacement(current: &RegistryString, text: &str) -> RegistryString {
    let kind = if current.kind == RegistryStringKind::Expandable || text.contains('%') {
        RegistryStringKind::Expandable
    } else {
        RegistryStringKind::Plain
    };
    RegistryString {
        kind,
        text: text.to_owned(),
    }
}

#[cfg(test)]
mod tests;
