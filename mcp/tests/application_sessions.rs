//! Purpose: Real MCP -> private FFM -> canonical application -> filesystem workflows.
//!
//! CI builds both binaries and explicitly runs these ignored artifact-dependent tests.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::redundant_pub_crate,
    clippy::panic,
    reason = "MCP integration test in active development — artifact-dependent, not yet fully integrated"
)]

#[path = "application_sessions/bdd.rs"]
mod bdd;
#[path = "application_sessions/boundaries.rs"]
mod boundaries;
#[path = "application_sessions/catalog.rs"]
mod catalog;
#[path = "application_sessions/client.rs"]
mod client;
#[path = "application_sessions/editing.rs"]
mod editing;
#[path = "application_sessions/terminal.rs"]
mod terminal;
