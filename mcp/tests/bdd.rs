//! Purpose: Runs BDD coverage for the MCP server JSON-RPC tool behavior.
//!
//! `bdd/world.rs` holds a scenario's state and `bdd/server.rs` the spawned
//! `ffm-mcp` every tool call talks to. The step definitions are grouped by
//! feature area in the `given_*`, `when_*`, `then_*` and `keepass_steps`
//! modules; cucumber registers them wherever they live.
#![allow(
    clippy::unused_async,
    clippy::needless_pass_by_ref_mut,
    clippy::needless_pass_by_value,
    reason = "cucumber dictates async steps over `&mut World`; the harness passes JSON by value"
)]

use cucumber::World as _;
use world::McpWorld;

#[path = "bdd/given_fixtures.rs"]
mod given_fixtures;
#[path = "bdd/keepass_steps.rs"]
mod keepass_steps;
#[path = "bdd/server.rs"]
mod server;
#[path = "bdd/then_files.rs"]
mod then_files;
#[path = "bdd/then_listings.rs"]
mod then_listings;
#[path = "bdd/then_results.rs"]
mod then_results;
#[path = "bdd/when_directories.rs"]
mod when_directories;
#[path = "bdd/when_files.rs"]
mod when_files;
#[path = "bdd/when_host.rs"]
mod when_host;
#[path = "bdd/when_search.rs"]
mod when_search;
#[path = "bdd/when_transfers.rs"]
mod when_transfers;
#[path = "bdd/world.rs"]
mod world;

// ─── Main ───────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    McpWorld::filter_run("tests/features", |feature, rule, scenario| {
        cfg!(unix)
            || !feature
                .tags
                .iter()
                .chain(rule.into_iter().flat_map(|rule| &rule.tags))
                .chain(scenario.tags.iter())
                .any(|tag| tag == "unix")
    })
    .await;
}
