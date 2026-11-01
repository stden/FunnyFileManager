//! Purpose: Bounded output capture and nonblocking remote input delivery.

use super::{
    master::Master,
    state::{Captured, Generation},
    HeadlessHost,
};
use std::{
    io::{self, Read},
    sync::{
        atomic::AtomicBool,
        mpsc::{self, Receiver, SyncSender},
        Arc, Mutex,
    },
    time::Duration,
};

mod capture;
mod input;
mod readiness;
mod task;
use super::responses::Reply;
use capture::Capture;
pub(super) use input::write;

pub(super) struct Output {
    finished: Receiver<io::Result<()>>,
    captured: Arc<Mutex<Captured>>,
    task: task::Task,
}

pub(super) fn read(
    mut reader: Box<dyn Read + Send>,
    host: HeadlessHost,
    generation: Generation,
    replies: SyncSender<Reply>,
    master: &Master,
) -> io::Result<Output> {
    let (sender, receiver) = mpsc::channel();
    let captured = Arc::new(Mutex::new(Captured::default()));
    let mut capture = Capture::new(host, generation, Arc::clone(&captured), replies);
    #[cfg(unix)]
    let ready = readiness::Reader::new(readiness::Lease::new(master))?;
    #[cfg(not(unix))]
    let ready = readiness::Lease::new(master);
    let task = task::Task::spawn(move |stop| {
        let result = read_all(&mut *reader, &mut capture, ready, &stop);
        // The action may already have ended after its bounded drain grace.
        drop(sender.send(result));
    })?;
    Ok(Output {
        finished: receiver,
        captured,
        task,
    })
}

fn read_all(
    reader: &mut dyn Read,
    capture: &mut Capture,
    ready: readiness::Capture,
    stop: &AtomicBool,
) -> io::Result<()> {
    #[cfg(unix)]
    let mut ready = ready;
    #[cfg(not(unix))]
    let _ = (ready, stop);
    let mut buffer = [0; 8192];
    loop {
        #[cfg(unix)]
        if stop.load(std::sync::atomic::Ordering::Acquire) || !capture.active() {
            return Ok(());
        }
        #[cfg(unix)]
        if !ready.wait()? {
            continue;
        }
        match reader.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(count) => capture.record(&buffer, count)?,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
                ) => {}
            // Unix PTYs report EIO when their last slave closes.
            #[cfg(unix)]
            Err(error) if error.raw_os_error() == Some(5) => return Ok(()),
            Err(error) => return Err(error),
        }
    }
}

pub(super) fn collect(mut output: Output) -> io::Result<Vec<u8>> {
    match output.finished.recv_timeout(Duration::from_millis(200)) {
        Ok(result) => result?,
        // Descendants can retain a PTY after their original program exits.
        Err(mpsc::RecvTimeoutError::Timeout) => {}
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            return Err(io::Error::other("Private terminal reader stopped"))
        }
    }
    output.task.close();
    let bytes = output
        .captured
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .bytes();
    Ok(bytes)
}
