//! Purpose: Surface capture-worker failures instead of returning partial success.

use super::*;

#[cfg(unix)]
mod reader;

fn output(result: Option<io::Result<()>>) -> io::Result<Output> {
    let (sender, finished) = mpsc::channel();
    if let Some(result) = result {
        sender.send(result).map_err(io::Error::other)?;
    }
    drop(sender);
    Ok(Output {
        finished,
        captured: Arc::new(Mutex::new(Captured::default())),
        task: task::Task::spawn(|_| {})?,
    })
}

#[test]
fn collecting_a_disconnected_worker_reports_reader_stopped() {
    let error = collect(output(None).unwrap()).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert_eq!(error.to_string(), "Private terminal reader stopped");
}

#[test]
fn collecting_a_failed_worker_preserves_the_read_error() {
    let result = Err(io::Error::new(
        io::ErrorKind::BrokenPipe,
        "fixture read failed",
    ));
    let error = collect(output(Some(result)).unwrap()).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    assert_eq!(error.to_string(), "fixture read failed");
}
