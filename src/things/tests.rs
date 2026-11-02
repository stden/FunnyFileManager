//! Unit tests: the catalog's settings, its listing shapes, the sessions its
//! machines become, the filter and the request's header. None reaches a network.

mod client;
mod config;
mod filter;
mod listing;
mod mapping;

use super::Machine;

/// A machine with `name` and `hostname`, a unique id and nothing else.
pub(super) fn machine(name: &str, hostname: &str) -> Machine {
    Machine {
        id: format!("id-{name}"),
        name: name.to_owned(),
        hostname: hostname.to_owned(),
        ..Machine::default()
    }
}
