//! Unit tests: the catalog's settings, its listing shapes, the sessions its
//! machines become, the filter and the request's header. None reaches a network.

mod cli;
mod cli_errors;
mod client;
mod config;
mod filter;
mod graphql;
mod listing;
mod mapping;
mod patch;
mod patch_rpc;

use std::{ffi::OsString, sync::MutexGuard};

use super::Machine;

/// Holds the global environment lock once across multiple variables and restores them on drop.
pub(super) struct EnvVarsGuard {
    _lock: MutexGuard<'static, ()>,
    previous: Vec<(&'static str, Option<OsString>)>,
}

impl EnvVarsGuard {
    pub(super) fn set_all(vars: &[(&'static str, &str)]) -> Self {
        let lock = crate::test_support::env_lock();
        let previous = vars
            .iter()
            .map(|(name, value)| {
                let prev = std::env::var_os(name);
                std::env::set_var(name, value);
                (*name, prev)
            })
            .collect();
        Self {
            _lock: lock,
            previous,
        }
    }
}

impl Drop for EnvVarsGuard {
    fn drop(&mut self) {
        for (name, prev) in &self.previous {
            match prev {
                Some(val) => std::env::set_var(name, val),
                None => std::env::remove_var(name),
            }
        }
    }
}

/// A machine with `name` and `hostname`, a unique id and nothing else.
pub(super) fn machine(name: &str, hostname: &str) -> Machine {
    Machine {
        id: format!("id-{name}"),
        name: name.to_owned(),
        hostname: hostname.to_owned(),
        ..Machine::default()
    }
}
