//! Purpose: Throttling, nesting and painter dispatch for long directory reads.

use std::cell::RefCell;
use std::rc::Rc;

use super::tests::Recorder;
use super::*;

#[derive(Clone, Default)]
struct ReadRecorder(Rc<RefCell<Vec<DirectoryRead>>>);

impl Painter for ReadRecorder {
    fn paint(&mut self, _snapshot: &Snapshot) {}

    fn paint_directory_read(&mut self, read: &DirectoryRead) {
        self.0.borrow_mut().push(*read);
    }
}

impl ReadRecorder {
    fn install(interval: Duration) -> (Self, Installed) {
        let recorder = Self::default();
        let installed = install_with(Box::new(recorder.clone()), interval);
        (recorder, installed)
    }

    fn frames(&self) -> Vec<DirectoryRead> {
        self.0.borrow().clone()
    }
}

#[test]
fn directory_read_paints_each_due_count_and_marks_screen_painted() {
    let (recorder, _installed) = ReadRecorder::install(Duration::ZERO);
    let header = Some(PanelHeader {
        x: 60,
        y: 1,
        width: 60,
    });
    let mut run = begin_directory_read(header);
    assert!(run.step());
    assert!(run.step());
    drop(run);
    assert_eq!(
        recorder.frames(),
        vec![
            DirectoryRead { files: 1, header },
            DirectoryRead { files: 2, header },
        ]
    );
    assert!(take_painted());
}

#[test]
fn fast_directory_read_never_paints_before_interval() {
    let (recorder, _installed) = ReadRecorder::install(Duration::from_secs(3600));
    let mut run = begin_directory_read(None);
    assert!(!run.step());
    assert!(!run.step());
    drop(run);
    assert_eq!(recorder.frames().len(), 0);
    assert!(!take_painted());
}

#[test]
fn directory_read_without_a_painter_is_silent() {
    let mut run = begin_directory_read(None);
    assert!(!run.step());
    drop(run);
    assert!(!take_painted());
}

#[test]
fn nested_directory_reads_leave_the_outer_progress_alone() {
    let (recorder, _installed) = ReadRecorder::install(Duration::ZERO);
    let mut outer = begin_directory_read(None);
    let mut inner = begin_directory_read(Some(PanelHeader {
        x: 0,
        y: 1,
        width: 40,
    }));
    assert!(!inner.step());
    drop(inner);
    assert!(outer.step());
    drop(outer);
    assert_eq!(
        recorder.frames(),
        vec![DirectoryRead {
            files: 1,
            header: None
        }]
    );

    let transfer = begin(Operation::Move, &[PathBuf::from("a")]);
    let mut during_transfer = begin_directory_read(None);
    assert!(!during_transfer.step());
    drop(during_transfer);
    drop(transfer);
}

#[test]
fn default_painter_ignores_directory_read_and_transfer_snapshot() {
    let (recorder, _installed) = Recorder::install();
    let mut run = begin_directory_read(None);
    assert!(run.step());
    drop(run);
    assert_eq!(recorder.frames().len(), 0);

    let mut read_only = ReadRecorder::default();
    read_only.paint(&Snapshot {
        operation: Operation::Copy,
        phase: Phase::Working,
        current: PathBuf::new(),
        transfer: None,
        done: Tally::default(),
        total: Tally::default(),
        elapsed: Duration::ZERO,
    });
    assert_eq!(read_only.frames().len(), 0);
}
