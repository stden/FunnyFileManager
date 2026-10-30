//! Purpose: Background worker thread for non-blocking network latency and DNS
//! probes (`"ffm-net-probe"`), keeping socket/ICMP waits off the UI thread.

use std::mem;
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::worker::Flight;
use crate::application::Redraw;
use crate::diagnostics::log_error;
use crate::models::tasks::{NetProber, ProbeOutcome, ProbeTarget};
use crate::models::{TaskHostSlot, TaskList, TaskSession};
use crate::performance::Performance;

const THREAD_NAME: &str = "ffm-net-probe";
const PROBE_ACTION: &str = "Background/Tasks/NetProbe";

type Batch = Vec<ProbeTarget>;
type Answers = Vec<(ProbeTarget, ProbeOutcome)>;

#[derive(Default)]
pub(super) struct NetProbeRunner {
    state: State,
}

#[derive(Default)]
enum State {
    #[default]
    Idle,
    Running(ProbeWorker),
    Abandoned(JoinHandle<()>),
    Disabled(TaskSession),
}

struct ProbeWorker {
    session: TaskSession,
    requests: SyncSender<Batch>,
    results: Receiver<Answers>,
    handle: JoinHandle<()>,
    flight: Flight,
}

impl NetProbeRunner {
    pub(super) fn retire(&mut self) {
        self.state = match mem::take(&mut self.state) {
            State::Running(ProbeWorker {
                handle,
                flight: Flight::Since(_),
                ..
            }) => State::Abandoned(handle),
            State::Running(ProbeWorker {
                flight: Flight::Idle,
                ..
            }) => State::Idle,
            kept @ (State::Idle | State::Abandoned(_) | State::Disabled(_)) => kept,
        };
    }

    pub(super) fn pump(
        &mut self,
        list: &mut TaskList,
        host: &TaskHostSlot,
        performance: &mut Performance,
        now: Instant,
    ) -> Redraw {
        if matches!(&self.state, State::Running(w) if w.session != list.session()) {
            self.retire();
        }
        let arrived = self.drain(list, performance, now);
        if list.net_diagnostics().rows().is_empty() || !self.ensure_worker(list.session(), host) {
            return arrived;
        }
        self.send_due(list, now);
        arrived
    }

    fn drain(
        &mut self,
        list: &mut TaskList,
        performance: &mut Performance,
        now: Instant,
    ) -> Redraw {
        let State::Running(worker) = &mut self.state else {
            return Redraw::Unchanged;
        };
        let outcomes = match worker.results.try_recv() {
            Ok(outcomes) => outcomes,
            Err(TryRecvError::Empty) => return Redraw::Unchanged,
            Err(TryRecvError::Disconnected) => {
                self.state = State::Disabled(list.session());
                return Redraw::Unchanged;
            }
        };
        if let Flight::Since(sent) = mem::replace(&mut worker.flight, Flight::Idle) {
            let round_trip = now.saturating_duration_since(sent);
            performance.record(PROBE_ACTION, round_trip, Duration::ZERO);
        }
        list.apply_probe_results(&outcomes, now);
        Redraw::Needed
    }

    fn ensure_worker(&mut self, session: TaskSession, host: &TaskHostSlot) -> bool {
        match &self.state {
            State::Running(worker) => return worker.flight == Flight::Idle,
            State::Disabled(disabled) if *disabled == session => return false,
            State::Abandoned(handle) if !handle.is_finished() => return false,
            State::Idle | State::Abandoned(_) | State::Disabled(_) => {}
        }
        let Some(prober) = host.host().and_then(|h| h.net_prober()) else {
            self.state = State::Disabled(session);
            return false;
        };
        match spawn_prober(prober, session) {
            Ok(worker) => {
                self.state = State::Running(worker);
                true
            }
            Err(error) => {
                log_error(&format!("Task list: cannot start net prober: {error}"));
                self.state = State::Disabled(session);
                false
            }
        }
    }

    fn send_due(&mut self, list: &mut TaskList, now: Instant) {
        let State::Running(worker) = &mut self.state else {
            return;
        };
        if worker.flight != Flight::Idle {
            return;
        }
        let Some(targets) = list.take_probe_request(now) else {
            return;
        };
        match worker.requests.try_send(targets) {
            Ok(()) | Err(TrySendError::Full(_)) => worker.flight = Flight::Since(now),
            Err(TrySendError::Disconnected(_)) => self.state = State::Disabled(list.session()),
        }
    }
}

fn spawn_prober(prober: Box<dyn NetProber>, session: TaskSession) -> std::io::Result<ProbeWorker> {
    let (requests, incoming) = mpsc::sync_channel(1);
    let (outgoing, results) = mpsc::sync_channel(1);
    let handle = thread::Builder::new()
        .name(THREAD_NAME.to_owned())
        .spawn(move || serve_probes(prober, &incoming, &outgoing))?;
    Ok(ProbeWorker {
        session,
        requests,
        results,
        handle,
        flight: Flight::Idle,
    })
}

fn serve_probes(
    mut prober: Box<dyn NetProber>,
    requests: &Receiver<Batch>,
    results: &SyncSender<Answers>,
) {
    while let Ok(targets) = requests.recv() {
        let answers = prober.probe(&targets);
        if results.send(answers).is_err() {
            break;
        }
    }
}
