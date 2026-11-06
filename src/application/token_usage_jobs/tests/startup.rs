//! Purpose: Worker startup failure publishes a provider report without a stuck request.
use super::*;

#[test]
fn a_rejected_worker_reports_each_provider_as_unavailable() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    crate::ops::token_usage::open(&mut app);
    let mut jobs = UsageJobs::default();
    FORCE_SPAWN_FAILURE.set(true);
    let redraw = jobs.pump_with(&mut app, fixture);
    FORCE_SPAWN_FAILURE.set(false);
    assert_eq!(redraw, Redraw::Needed);
    assert!(jobs.updated.is_some());
    let Overlay::TokenUsage(panel) = &app.overlay else {
        panic!("usage panel");
    };
    for provider in Provider::ALL {
        assert!(panel
            .viewer
            .lines
            .iter()
            .any(|line| line.contains(provider.label())
                && line.contains(UsageError::Worker.message())));
    }
}
