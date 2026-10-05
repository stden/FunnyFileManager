//! Test adapters and fixtures, excluded from ordinary production builds.

mod backend;
mod executable;
pub use executable::write_executable;
#[cfg(any(test, feature = "test-helpers"))]
mod environment;
#[cfg(any(test, feature = "test-helpers"))]
mod lsp;
pub use backend::WritableTestBackend;
#[cfg(any(test, feature = "test-helpers"))]
pub use environment::*;
#[cfg(any(test, feature = "test-helpers"))]
pub use lsp::{program_file, set_lsp_probe};
#[cfg(any(test, feature = "test-helpers"))]
mod settings;
#[cfg(any(test, feature = "test-helpers"))]
pub use settings::restore_settings;
#[cfg(any(test, feature = "test-helpers"))]
mod mcp_install;
#[cfg(all(unix, any(test, feature = "test-helpers")))]
mod permissions;
#[cfg(all(unix, any(test, feature = "test-helpers")))]
pub use permissions::ModeGuard;
#[cfg(any(test, feature = "test-helpers"))]
pub mod processes;
#[cfg(any(test, feature = "test-helpers"))]
pub use mcp_install::{file_identity, ClaudeBehaviour, ClaudeEmulator, FakeMachine};

#[cfg(test)]
pub(crate) mod disk_cleanup;
#[cfg(test)]
mod events;
#[cfg(test)]
pub(crate) mod lsp_machine;
#[cfg(test)]
pub(crate) mod netbox;
#[cfg(test)]
#[cfg(unix)]
pub(crate) mod sleeper;
#[cfg(test)]
pub(crate) mod task_list;
#[cfg(test)]
pub(crate) use events::*;

#[cfg(coverage)]
mod coverage;
#[cfg(coverage)]
pub use coverage::{coverage_touch_backend, coverage_touch_logs};

#[cfg(test)]
mod tests;
