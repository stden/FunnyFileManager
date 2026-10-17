//! Purpose: FFM's side of the administrator helper on Windows: start it
//! once — the UAC prompt — keep the pipe to it for the rest of the session,
//! and pass it requests; and the helper's own start, which checks it was
//! started by the FFM it talks to.

use std::cell::RefCell;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, BufReader};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use super::protocol::{self, Outcome, Reply};
use super::HELPER_FLAG;
use crate::models::ElevatedRetry;
use crate::platform::{self, ElevatedProcess};

/// How long a helper Windows started may take to connect.
const CONNECT_PATIENCE: Duration = Duration::from_secs(30);
/// How long the helper waits for FFM's pipe.
const PIPE_PATIENCE: Duration = Duration::from_secs(10);

/// The pipe to the running helper.
struct Link {
    reader: BufReader<File>,
    writer: File,
    /// Kept so the handle lives as long as the session.
    _helper: ElevatedProcess,
}

thread_local! {
    static SESSION: RefCell<Option<Link>> = const { RefCell::new(None) };
}

pub(super) fn active() -> bool {
    SESSION.with(|session| session.borrow().is_some())
}

/// Send `retry` to the helper — started first, which asks the user, when
/// the session has none — and wait for its outcome, drawing its progress.
pub(super) fn exchange(retry: &ElevatedRetry) -> io::Result<Outcome> {
    SESSION.with(|session| {
        let mut session = session.borrow_mut();
        if session.is_none() {
            *session = Some(start()?);
        }
        let link = session
            .as_mut()
            .ok_or_else(|| io::Error::other("no helper"))?;
        let outcome = talk(link, retry);
        if outcome.is_err() {
            // A helper that stopped answering is not used again; the next
            // refusal asks Windows for a new one.
            *session = None;
        }
        outcome
    })
}

fn talk(link: &mut Link, retry: &ElevatedRetry) -> io::Result<Outcome> {
    protocol::send(&mut link.writer, retry)?;
    loop {
        match protocol::receive::<Reply>(&mut link.reader)? {
            Some(Reply::Progress(snapshot)) => crate::progress::relay(&snapshot),
            Some(Reply::Finished(outcome)) => return Ok(outcome),
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "the administrator helper ended",
                ))
            }
        }
    }
}

/// Create the pipe, start the helper as an administrator, and let it in
/// only if the process that connects is the one Windows started.
fn start() -> io::Result<Link> {
    let name = pipe_name();
    let pipe = platform::create_pipe(&name)?;
    let parameters = format!("{HELPER_FLAG} {name} {}", std::process::id());
    let helper =
        platform::launch_as_administrator(&std::env::current_exe()?, OsStr::new(&parameters))?;
    wait_for_helper(&pipe, &helper, &name)?;
    if platform::client_process_id(&pipe)? != helper.id() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "another program answered for the administrator helper",
        ));
    }
    Ok(Link {
        reader: BufReader::new(pipe.try_clone()?),
        writer: pipe,
        _helper: helper,
    })
}

/// Wait on a thread for the helper to connect, giving up when it ends or
/// takes too long; then the waiting thread is let go by connecting to it.
fn wait_for_helper(pipe: &File, helper: &ElevatedProcess, name: &str) -> io::Result<()> {
    let waiting = pipe.try_clone()?;
    let (sender, connected) = mpsc::channel();
    std::thread::spawn(move || drop(sender.send(platform::accept(&waiting))));
    let deadline = Instant::now() + CONNECT_PATIENCE;
    loop {
        match connected.recv_timeout(Duration::from_millis(100)) {
            Ok(result) => return result,
            Err(RecvTimeoutError::Timeout) if helper.has_exited() || Instant::now() >= deadline => {
                drop(platform::open_pipe(name, Duration::from_secs(1)));
                return Err(io::Error::other("the administrator helper did not start"));
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(io::Error::other("the administrator helper did not start"));
            }
        }
    }
}

/// A pipe name nobody else can guess: the process id and 128 random bits.
fn pipe_name() -> String {
    use std::hash::{BuildHasher, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    let random = || {
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        hasher.write_u128(nanos);
        hasher.finish()
    };
    format!(
        "ffm-elevated-{}-{:016x}{:016x}",
        std::process::id(),
        random(),
        random()
    )
}

/// The helper's start: connect to FFM's pipe `name`, check that FFM process
/// `parent` is at its other end, and serve it until it closes the pipe.
pub(super) fn serve_parent(name: &OsString, parent: &OsString) -> io::Result<()> {
    let invalid = || io::Error::from(io::ErrorKind::InvalidInput);
    let name = name.to_str().ok_or_else(invalid)?;
    if !name.starts_with("ffm-elevated-")
        || !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
    {
        return Err(invalid());
    }
    let parent: u32 = parent
        .to_str()
        .and_then(|text| text.parse().ok())
        .ok_or_else(invalid)?;
    let pipe = platform::open_pipe(name, PIPE_PATIENCE)?;
    if platform::server_process_id(&pipe)? != parent {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the pipe does not belong to the FFM that started this helper",
        ));
    }
    super::helper::serve(BufReader::new(pipe.try_clone()?), pipe)
}
