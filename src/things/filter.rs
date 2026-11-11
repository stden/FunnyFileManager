//! Purpose: The dialog's filter: a case-insensitive match over the fields a
//! person searches by.

use super::Machine;
use crate::casefold::CaseNeedle;

/// Whether `machine` matches `filter`: the filter, ignoring case, is part of its
/// name, hostname, IP address, environment, location or one of its tags. An
/// empty filter matches every machine.
#[must_use]
pub fn matches(machine: &Machine, filter: &str) -> bool {
    filter.is_empty() || matches_needle(machine, &CaseNeedle::new(filter))
}

#[must_use]
pub fn matches_needle(machine: &Machine, needle: &CaseNeedle) -> bool {
    searched(machine).any(|field| needle.contains_in(field))
}

fn searched(machine: &Machine) -> impl Iterator<Item = &str> {
    [machine.name.as_str(), machine.hostname.as_str()]
        .into_iter()
        .chain(machine.ip_address.as_deref())
        .chain(machine.environment.as_deref())
        .chain(machine.location.as_deref())
        .chain(machine.tags.iter().map(String::as_str))
}
