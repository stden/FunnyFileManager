//! Gherkin acceptance tests against production dispatch, workers and rendering.

#![expect(
    clippy::future_not_send,
    reason = "Cucumber's generated constructor awaits a world owning a single-threaded test backend and clipboard"
)]

use cucumber::{writer::Stats as _, World as _};

mod case;
mod common;
mod documents;
mod editor;
mod image;
mod lsp;
mod lsp_fixture;
mod network;
mod network_drives;
mod network_worker;
mod panels;
mod queue;
mod quick_search;
mod samples;
mod server_profiles;
mod space_analysis;
mod statistics;
mod tasks;
mod temp_cleanup;
mod things_catalog;
mod token_usage;
mod viewer;

use case::Case;

#[derive(cucumber::World, Default)]
struct BehaviorWorld {
    case: Option<Case>,
    mounts: Option<network_drives::Fixture>,
    profiles: Option<server_profiles::Fixture>,
    usage: Option<token_usage::Fixture>,
    temp_cleanup: Option<temp_cleanup::Fixture>,
    things: Option<things_catalog::Fixture>,
}

impl std::fmt::Debug for BehaviorWorld {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BehaviorWorld")
            .field("screen", &self.case.as_ref().map(|case| case.screen.text()))
            .finish_non_exhaustive()
    }
}

impl BehaviorWorld {
    fn case(&mut self) -> &mut Case {
        self.case
            .as_mut()
            .expect("the workspace Given step runs first")
    }
}

#[test]
fn recent_features_bdd() {
    std::thread::Builder::new()
        .name("recent-features-bdd".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(run_scenarios)
        .unwrap()
        .join()
        .unwrap();
}

fn run_scenarios() {
    let writer = futures::executor::block_on(
        BehaviorWorld::cucumber()
            .with_cli(cucumber::cli::Opts::<_, _, _, cucumber::cli::Empty>::default())
            .max_concurrent_scenarios(1)
            .fail_on_skipped()
            .run(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/behavior_features"
            )),
    );
    assert!(
        !writer.execution_has_failed(),
        "BDD scenarios must all pass"
    );
    assert_eq!(writer.skipped_steps(), 0, "every step must be implemented");
    assert!(
        writer.passed_steps() > 0,
        "BDD feature files must be discovered"
    );
}
