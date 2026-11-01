//! BDD against the production event loop and the actual rendered terminal cells.

use cucumber::World as _;

#[path = "support/editor_backend.rs"]
mod editor_backend;
#[path = "tui_bdd/mod.rs"]
mod tui_bdd;
use tui_bdd::TuiWorld;

#[test]
fn the_shared_backend_forwards_writes_cursor_and_window_size() {
    use ratatui::backend::Backend;
    use std::io::Write;

    let mut backend = editor_backend::SharedBackend::new(4, 2);
    assert!(backend.write(b"hi").is_ok());
    assert!(Write::flush(&mut backend).is_ok());
    assert!(backend.hide_cursor().is_ok());
    assert!(backend.get_cursor_position().is_ok());
    assert!(backend.clear().is_ok());
    assert!(backend.window_size().is_ok());
}

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
