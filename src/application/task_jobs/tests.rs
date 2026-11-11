//! Purpose: Tests for the sampling controller, driven with synthetic
//! instants over a gated fixture reader: nothing runs while the list is
//! closed, one request is in flight at a time, and a new session starts
//! afresh; and the fixture the stall and failure tests share.

use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use tempfile::TempDir;

use super::TaskJobs;
use crate::application::Redraw;
use crate::models::{
    App, CpuMeter, Overlay, Panel, Pid, ProcessHost, Request, Sample, SampleError, Schedule, Table,
    TaskList, STALL_LIMIT, WARM_UP,
};
use crate::prompt::Privilege;
use crate::test_support::processes::{one_core, process, FixtureHost, Gate};

/// How long a test waits for the worker thread, in real time.
pub(super) const WAIT: Duration = Duration::from_secs(5);

/// A millisecond short of `limit`.
pub(super) const fn just_before(limit: Duration) -> Duration {
    limit.saturating_sub(Duration::from_millis(1))
}

pub(super) struct Fixture {
    pub(super) app: App,
    pub(super) jobs: TaskJobs,
    _root: TempDir,
}

impl Fixture {
    /// An app with nothing open and `host` installed, if any.
    pub(super) fn new(host: Option<Arc<dyn ProcessHost>>) -> Self {
        let root = TempDir::new().expect("temporary directory");
        let panel = || Panel::from_entries(root.path().to_path_buf(), Vec::new());
        let mut app = App::with_panels(panel(), panel());
        if let Some(host) = host {
            app.task_host.install(host);
        }
        Self {
            app,
            jobs: TaskJobs::default(),
            _root: root,
        }
    }

    /// An app with a gated host installed, and the gate.
    pub(super) fn gated() -> (Self, FixtureHost, Gate) {
        let (host, gate) = FixtureHost::gated();
        (Self::new(Some(Arc::new(host.clone()))), host, gate)
    }

    /// Opens the task list afresh: a new session.
    pub(super) fn open(&mut self) {
        let session = self.app.task_host.next_session();
        let own = Pid::new(900).expect("a PID");
        self.app.set_overlay(Overlay::Tasks(Box::new(TaskList::new(
            session,
            own,
            Privilege::User,
        ))));
    }

    pub(super) fn pump(&mut self, now: Instant) -> Redraw {
        self.jobs.pump_at(&mut self.app, now)
    }

    /// Pumps at `now` until an answer is applied or the worker is lost.
    pub(super) fn pump_until_needed(&mut self, now: Instant) {
        let deadline = Instant::now() + WAIT;
        while self.pump(now) == Redraw::Unchanged {
            assert!(Instant::now() < deadline, "nothing arrived");
            thread::yield_now();
        }
    }

    pub(super) fn list(&self) -> &TaskList {
        match &self.app.overlay {
            Overlay::Tasks(list) => list,
            _ => panic!("the task list is open"),
        }
    }

    pub(super) fn table(&self) -> &Table {
        self.list().table()
    }
}

/// The sample every gate lets through.
pub(super) fn answer() -> Result<Sample, SampleError> {
    Ok(one_core(vec![process(
        Pid::new(42).expect("a PID"),
        "firefox",
    )]))
}

/// Pumps at `now` until `host` has handed out `count` readers.
pub(super) fn pump_until_readers(
    fixture: &mut Fixture,
    host: &FixtureHost,
    count: usize,
    now: Instant,
) {
    let deadline = Instant::now() + WAIT;
    while host.readers_opened() < count {
        assert!(Instant::now() < deadline, "no reader {count}");
        fixture.pump(now);
        thread::yield_now();
    }
}

/// An opened list whose first sample, asked for at `t0`, has been applied.
pub(super) fn sampled(t0: Instant) -> (Fixture, FixtureHost, Gate) {
    let (mut fixture, host, gate) = Fixture::gated();
    fixture.open();
    fixture.pump(t0);
    gate.open(answer());
    fixture.pump_until_needed(t0);
    (fixture, host, gate)
}

#[test]
fn nothing_runs_while_the_list_is_closed() {
    let (mut fixture, host, gate) = Fixture::gated();
    let t0 = Instant::now();
    for step in 0..10 {
        assert_eq!(fixture.pump(t0 + STALL_LIMIT * step), Redraw::Unchanged);
    }
    assert_eq!((host.readers_opened(), gate.entered()), (0, 0));
}

#[test]
fn the_first_request_goes_out_at_once_and_no_second_while_one_is_in_flight() {
    let (mut fixture, host, gate) = Fixture::gated();
    fixture.open();
    let t0 = Instant::now();
    assert_eq!(fixture.pump(t0), Redraw::Unchanged);
    assert!(gate.wait_for_entries(1, WAIT), "the first sample began");
    let waiting = Schedule::Waiting {
        since: t0,
        request: Request::Explicit,
    };
    assert_eq!(fixture.list().refresh().schedule(), waiting);
    for later in [Duration::from_secs(1), Duration::from_secs(4)] {
        assert_eq!(fixture.pump(t0 + later), Redraw::Unchanged);
    }
    gate.open(answer());
    fixture.pump_until_needed(t0 + Duration::from_secs(4));
    assert!(fixture.list().snapshot().is_some());
    assert!(
        !gate.wait_for_entries(2, Duration::from_millis(100)),
        "nothing more was asked for before the warm-up"
    );
    assert_eq!(host.readers_opened(), 1);
    fixture.pump(t0 + Duration::from_secs(4) + WARM_UP);
    assert!(gate.wait_for_entries(2, WAIT), "the next sample began");
}

#[test]
fn a_sample_round_trip_is_recorded() {
    let (fixture, _host, _gate) = sampled(Instant::now());
    let report = fixture.app.performance.report().join("\n");
    assert!(report.contains("Background/Tasks/Sample"), "{report}");
}

/// Reopening the list is a new session: its worker, reader and baselines
/// are new, so its first CPU shares are pending.
#[test]
fn a_new_session_retires_the_idle_worker_and_starts_afresh() {
    let t0 = Instant::now();
    let (mut fixture, host, gate) = sampled(t0);
    fixture.open();
    fixture.pump(t0);
    assert_eq!(host.readers_opened(), 2);
    assert!(gate.wait_for_entries(2, WAIT));
    gate.open(answer());
    fixture.pump_until_needed(t0);
    let snapshot = fixture.list().snapshot().expect("a first snapshot");
    assert_eq!(snapshot.meters().cpu(), CpuMeter::Pending);
}
