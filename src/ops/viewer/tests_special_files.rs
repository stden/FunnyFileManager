//! Unit tests: F3 on things that are not regular files.
//!
//! Every non-regular file reports `len() == 0`, so the size guard never saw
//! them: `/dev/zero` grew a `Vec` until the OOM killer fired, and a
//! writer-less FIFO wedged the event loop inside `File::open` with the
//! terminal still in raw mode. Both cases are guarded by a timeout here, so a
//! regression fails the suite instead of hanging it.

use super::handle_view;

use crate::models::{App, Overlay};
use crate::test_support::{make_fifo, run_with_timeout};

use std::io;
use std::path::Path;
use std::time::Duration;

use tempfile::TempDir;

/// Longest F3 may take on a refused file. Generous enough for a loaded
/// machine, short enough that a blocking regression is caught in seconds.
const GUARD_TIMEOUT: Duration = Duration::from_secs(5);

/// Status line and "did an overlay open?" after F3 on `name` inside a fresh
/// temp dir prepared by `prepare`.
fn view_status_in_temp_dir<F>(name: &'static str, prepare: F) -> io::Result<(String, bool)>
where
    F: FnOnce(&Path) -> io::Result<()>,
{
    let temp = TempDir::new()?;
    prepare(&temp.path().join(name))?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;
    let index = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .ok_or_else(|| {
            io::Error::other("the panel lists special files, which is how the cursor gets onto one")
        })?;
    app.left_panel.select(Some(index));

    handle_view(&mut app)?;
    let opened = !matches!(app.overlay, Overlay::None);
    Ok((app.status_text(), opened))
}

#[test]
fn f3_on_a_writerless_fifo_is_refused_without_blocking() {
    let result = run_with_timeout(GUARD_TIMEOUT, || view_status_in_temp_dir("pipe", make_fifo))
        .expect("F3 on a writer-less FIFO must return, not block the event loop forever");

    let (status, opened) = result.expect("a FIFO is not a handler failure");
    assert!(
        status.starts_with("Cannot open") && status.ends_with("not a regular file"),
        "status was {status:?}"
    );
    assert!(!opened, "nothing should have been opened");
}

/// `/dev/zero` is endless and reports zero length — the exact shape the size
/// guard could not catch.
#[test]
fn f3_on_an_endless_device_node_is_refused_without_exhausting_memory() {
    if !Path::new("/dev/zero").exists() {
        return;
    }

    let result = run_with_timeout(GUARD_TIMEOUT, || {
        let mut app = App::new()?;
        app.left_panel.path = std::path::PathBuf::from("/dev");
        app.left_panel.load_entries()?;
        let index = app
            .left_panel
            .entries
            .iter()
            .position(|entry| entry.name == "zero")
            .expect("/dev lists `zero`");
        app.left_panel.select(Some(index));

        handle_view(&mut app)?;
        Ok::<_, io::Error>((app.status_text(), !matches!(app.overlay, Overlay::None)))
    })
    .expect("F3 on /dev/zero must return promptly instead of reading until it is killed");

    let (status, opened) = result.expect("a device node is not a handler failure");
    assert!(
        status.ends_with("not a regular file"),
        "status was {status:?}"
    );
    assert!(!opened, "nothing should have been opened");
}
