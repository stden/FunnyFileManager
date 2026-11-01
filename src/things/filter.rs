//! Purpose: The dialog's filter: a case-insensitive match over the fields a
//! person searches by.

use super::Machine;

/// Whether `machine` matches `filter`: the filter, ignoring case, is part of its
/// name, hostname, IP address, environment, location or one of its tags. An
/// empty filter matches every machine.
#[must_use]
pub fn matches(machine: &Machine, filter: &str) -> bool {
    let needle = filter.to_lowercase();
    needle.is_empty() || searched(machine).any(|field| field.to_lowercase().contains(&needle))
}

fn searched(machine: &Machine) -> impl Iterator<Item = &str> {
    [machine.name.as_str(), machine.hostname.as_str()]
        .into_iter()
        .chain(machine.ip_address.as_deref())
        .chain(machine.environment.as_deref())
        .chain(machine.location.as_deref())
        .chain(machine.tags.iter().map(String::as_str))
}
