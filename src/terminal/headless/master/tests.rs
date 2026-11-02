//! Purpose: A background slave holder cannot retain a completed capture worker or master.

use super::*;
use crate::terminal::headless::{state::Generation, streams, ExternalState, HeadlessHost};
use std::{
    io::Read,
    sync::{mpsc, Weak},
    time::{Duration, Instant},
};

mod contracts;

#[derive(Clone, Copy)]
enum End {
    Completed,
    Cancelled,
    Replaced,
    Dropped,
}

struct WatchedReader {
    reader: Box<dyn Read + Send>,
    dropped: mpsc::Sender<()>,
}

impl Read for WatchedReader {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.reader.read(bytes)
    }
}

impl Drop for WatchedReader {
    fn drop(&mut self) {
        let _delivery = self.dropped.send(());
    }
}

struct Fixture {
    host: HeadlessHost,
    generation: Generation,
    output: streams::Output,
    master: Weak<Mutex<Box<dyn MasterPty + Send>>>,
    dropped: mpsc::Receiver<()>,
    _slave: Box<dyn portable_pty::SlavePty + Send>,
}

fn fixture() -> io::Result<Fixture> {
    let pair = portable_pty::native_pty_system()
        .openpty(PtySize::default())
        .map_err(io::Error::other)?;
    let reader = pair.master.try_clone_reader().map_err(io::Error::other)?;
    let master = Master::new(pair.master);
    let weak = Arc::downgrade(&master.0);
    let (dropped, observed) = mpsc::channel();
    let host = HeadlessHost::new();
    let (input, _receiver) = mpsc::sync_channel(1);
    let generation = host.begin(input)?;
    let (replies, _responses) = mpsc::sync_channel(1);
    let reader = Box::new(WatchedReader { reader, dropped });
    let output = streams::read(reader, host.clone(), generation, replies, &master)?;
    drop(master);
    Ok(Fixture {
        host,
        generation,
        output,
        master: weak,
        dropped: observed,
        _slave: pair.slave,
    })
}

#[test]
fn retained_slave_does_not_keep_capture_resources_after_completion_or_cancel() -> io::Result<()> {
    for end in [End::Completed, End::Cancelled, End::Replaced, End::Dropped] {
        check(end)?;
    }
    Ok(())
}

fn check(end: End) -> io::Result<()> {
    let fixture = fixture()?;
    assert!(
        fixture.master.upgrade().is_some(),
        "reader retains its master lease"
    );
    prepare(&fixture, end)?;
    let started = Instant::now();
    match end {
        End::Dropped => drop(fixture.output),
        _ => assert_eq!(
            streams::collect(fixture.output)?,
            Vec::<u8>::new(),
            "the private fixture must produce no output"
        ),
    }
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "capture must stop within its drain/poll budget"
    );
    assert!(
        fixture.dropped.try_recv().is_ok(),
        "reader thread released its handle"
    );
    assert!(
        fixture.master.upgrade().is_none(),
        "reader released the last master lease"
    );
    // The slave stays owned until scope exit, as a background process can retain it.
    Ok(())
}

fn prepare(fixture: &Fixture, end: End) -> io::Result<()> {
    match end {
        End::Cancelled => fixture.host.cancel(),
        End::Replaced => {
            fixture
                .host
                .finish(fixture.generation, ExternalState::Exited { code: 0 });
            fixture.host.reset()?;
        }
        End::Completed | End::Dropped => {}
    }
    Ok(())
}
