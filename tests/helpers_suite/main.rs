//! Consolidated integration tests that require the `test-helpers` feature.

#[path = "../support/editor_backend.rs"]
mod editor_backend;

mod comprehensive_test;
mod coverage_100_test;
mod coverage_boost_test;
mod coverage_gaps_test;
mod e2e_test;
mod editor_test;
mod editor_workflow_test;
mod extra_coverage_test;
mod far_manager_hotkeys_test;
mod handlers_test;
mod highlight_render_test;
mod menu_commands_test;
mod mutation_killer_tests;
mod navigation_features_test;
mod quickview_test;
mod system_open_test;
mod targeted_coverage_tests;
mod wsl_workflow_test;
