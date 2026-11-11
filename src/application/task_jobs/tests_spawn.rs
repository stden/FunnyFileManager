//! A failed worker spawn is reported as an unavailable sampler.

use std::time::Instant;

use super::tests::Fixture;
use super::worker::FORCE_SPAWN_FAILURE;
use crate::application::Redraw;
use crate::models::{SampleError, Table};

#[test]
fn a_reader_thread_that_cannot_start_shows_the_sampler_unavailable() {
    let (mut fixture, _host, _gate) = Fixture::gated();
    fixture.open();

    FORCE_SPAWN_FAILURE.set(true);
    let redraw = fixture.pump(Instant::now());
    FORCE_SPAWN_FAILURE.set(false);

    assert_eq!(redraw, Redraw::Needed);
    assert!(matches!(
        fixture.table(),
        Table::Failed(SampleError::WorkerUnavailable)
    ));
}
