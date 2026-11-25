//! Purpose: Gherkin acceptance scenarios use the same real MCP subprocess client.
#![allow(
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    reason = "Cucumber step signatures own captured text and require a mutable world"
)]

#[path = "bdd/editor.rs"]
mod editor;
#[path = "bdd/shell.rs"]
mod shell;
#[path = "bdd/terminal.rs"]
mod terminal;
#[path = "bdd/world.rs"]
mod world;

use cucumber::{writer::Stats as _, World as _};
use world::SessionWorld;

#[tokio::test]
#[ignore = "requires FFM_APPLICATION_TEST_BINARY pointing to a separately built ffm executable"]
#[expect(
    clippy::excessive_nesting,
    reason = "cucumber after-hook requires Box::pin(async move { ... }) nesting; extracting would add boilerplate without clarity"
)]
async fn background_application_features() {
    let writer = SessionWorld::cucumber()
        .with_cli(cucumber::cli::Opts::<_, _, _, cucumber::cli::Empty>::default())
        .max_concurrent_scenarios(1)
        .after(|_, _, _, _, world| {
            Box::pin(async move {
                if let Some(world) = world {
                    world.finish().await;
                }
            })
        })
        .fail_on_skipped()
        .run(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/application_features"
        ))
        .await;
    assert!(
        !writer.execution_has_failed(),
        "MCP application BDD must pass"
    );
    assert_eq!(
        writer.skipped_steps(),
        0,
        "every application step is implemented"
    );
    assert!(
        writer.passed_steps() > 0,
        "application features must be discovered"
    );
}
