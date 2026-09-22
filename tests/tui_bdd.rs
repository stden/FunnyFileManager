//! BDD against the production event loop and the actual rendered terminal cells.

use cucumber::World as _;

#[path = "support/editor_backend.rs"]
mod editor_backend;
#[path = "tui_bdd/mod.rs"]
mod tui_bdd;
use tui_bdd::TuiWorld;

#[test]
fn tui_workflows() {
    futures::executor::block_on(
        TuiWorld::cucumber()
            .with_cli(cucumber::cli::Opts::<_, _, _, cucumber::cli::Empty>::default())
            .max_concurrent_scenarios(1)
            .before(|_, _, scenario, world| {
                world.name.clone_from(&scenario.name);
                Box::pin(async {})
            })
            .after(|_, _, _, _, world| {
                Box::pin(async move {
                    if let Some(world) = world {
                        world.finish();
                    }
                })
            })
            .fail_on_skipped()
            .run_and_exit("tests/tui_features"),
    );
}
