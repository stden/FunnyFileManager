//! Purpose: What the progress dialog is told during F5, F6 and F8, how often,
//! and that nothing is shown without a painter.

use std::rc::Rc;

use super::*;

/// Every snapshot it was asked to draw.
#[derive(Clone, Default)]
struct Recorder(Rc<RefCell<Vec<Snapshot>>>);

impl Painter for Recorder {
    fn paint(&mut self, snapshot: &Snapshot) {
        self.0.borrow_mut().push(snapshot.clone());
    }
}

impl Recorder {
    /// Installed, drawing every report.
    fn install() -> (Self, Installed) {
        let recorder = Self::default();
        let installed = install_with(Box::new(recorder.clone()), Duration::ZERO);
        (recorder, installed)
    }

    fn frames(&self) -> Vec<Snapshot> {
        self.0.borrow().clone()
    }

    fn last(&self) -> Snapshot {
        self.frames().last().cloned().expect("something was drawn")
    }
}

fn snapshot(done: Tally, total: Tally) -> Snapshot {
    Snapshot {
        operation: Operation::Copy,
        phase: Phase::Working,
        current: PathBuf::new(),
        done,
        total,
        elapsed: Duration::ZERO,
    }
}

const fn tally(files: u64, bytes: u64) -> Tally {
    Tally { files, bytes }
}

#[test]
fn the_bar_counts_bytes_once_there_are_bytes_and_files_otherwise() {
    assert_eq!(snapshot(tally(1, 250), tally(4, 1000)).permille(), 250);
    assert_eq!(snapshot(tally(1, 0), tally(4, 0)).permille(), 250);
    assert_eq!(snapshot(tally(0, 0), tally(0, 0)).permille(), 0);
    // A tree that grew while it was copied still stops at a full bar.
    assert_eq!(snapshot(tally(9, 2000), tally(4, 1000)).permille(), 1000);
}

#[test]
fn a_frame_waits_for_the_interval_after_the_start_and_after_the_last() {
    let start = Instant::now();
    let interval = Duration::from_millis(100);
    let mut throttle = Throttle::new(start);
    assert!(!throttle.due(start + Duration::from_millis(99), interval));
    assert!(throttle.due(start + interval, interval));
    throttle.last = Some(start + interval);
    assert!(!throttle.due(start + Duration::from_millis(150), interval));
    assert!(throttle.due(start + Duration::from_millis(200), interval));
}

#[test]
fn nothing_is_shown_without_a_painter() {
    let run = begin(Operation::Copy, &[PathBuf::from("x")]);
    assert!(!is_shown());
    entering(Path::new("y"));
    drop(run);
    assert!(!take_painted());
}

#[test]
fn a_frame_is_never_drawn_before_the_interval_has_passed() {
    let recorder = Recorder::default();
    let _installed = install_with(Box::new(recorder.clone()), Duration::from_secs(3600));
    let run = begin(Operation::Delete, &[]);
    entering(Path::new("a"));
    removed(Path::new("a"));
    drop(run);
    assert!(recorder.frames().is_empty());
    assert!(!take_painted());
}

#[test]
fn a_run_begun_inside_another_leaves_it_alone() {
    let (recorder, _installed) = Recorder::install();
    let outer = begin(Operation::Move, &[PathBuf::from("a")]);
    let inner = begin(Operation::Copy, &[]);
    drop(inner);
    assert!(is_shown(), "the inner run ended nothing");
    source_done();
    drop(outer);
    assert_eq!(recorder.last().operation, Operation::Move);
    assert!(!is_shown());
}

#[test]
fn an_uninstalled_painter_draws_nothing_more() {
    let (recorder, installed) = Recorder::install();
    drop(installed);
    let run = begin(Operation::Copy, &[]);
    entering(Path::new("x"));
    drop(run);
    assert!(recorder.frames().is_empty());
}

#[test]
fn the_operation_of_a_dialog_is_its_purpose() {
    assert_eq!(Operation::of(crate::copy::Purpose::Copy), Operation::Copy);
    assert_eq!(Operation::of(crate::copy::Purpose::Move), Operation::Move);
}
