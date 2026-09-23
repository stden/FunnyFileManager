//! Test adapters and fixtures, excluded from ordinary production builds.

mod backend;
mod executable;
pub use executable::write_executable;
#[cfg(any(test, feature = "test-helpers"))]
mod environment;
pub use backend::WritableTestBackend;
#[cfg(any(test, feature = "test-helpers"))]
pub use environment::*;

#[cfg(test)]
pub(crate) mod disk_cleanup;
#[cfg(test)]
mod events;
#[cfg(test)]
pub(crate) mod netbox;
#[cfg(test)]
pub(crate) use events::*;

#[cfg(coverage)]
mod coverage;
#[cfg(coverage)]
pub use coverage::{coverage_touch_backend, coverage_touch_logs};

#[cfg(test)]
mod tests;
