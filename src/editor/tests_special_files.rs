//! Unit tests: F4 on things that are not regular files — the twin of
//! `ops::viewer::tests_special_files`.
//!
//! F4 ran the identical `metadata().len() > MAX_FILE_SIZE.bytes()` guard, and
//! that guard is a no-op for every non-regular file (they all report length
//! zero). Opening a writer-less FIFO then blocked inside `File::open` forever
//! with the terminal still in raw mode. Both cases are timeout-guarded, so a
//! regression fails the suite instead of hanging it.

use super::handle_edit_builtin;
use super::tests::{build_app, select_entry};

use crate::models::{App, Overlay};
use crate::test_support::{make_fifo, run_with_timeout};

use std::io;
use std::path::Path;
use std::time::Duration;

use tempfile::TempDir;

/// Longest F4 may take on a refused file.
const GUARD_TIMEOUT: Duration = Duration::from_secs(5);

/// Status line and "did an overlay open?" after F4 on `name` inside a fresh
/// temp dir prepared by `prepare`.
fn edit_status_in_temp_dir<F>(name: &'static str, prepare: F) -> io::Result<(String, bool)>
where
    F: FnOnce(&Path) -> io::Result<()>,
{
    let temp = TempDir::new()?;
    prepare(&temp.path().join(name))?;

    let mut app = build_app(&temp);
    assert!(
        select_entry(&mut app.left_panel, name),
        "the panel lists special files, which is how the cursor gets onto one"
    );

    handle_edit_builtin(&mut app)?;
    let opened = !matches!(app.overlay, Overlay::None);
    Ok((app.status_text(), opened))
}

#[test]
fn f4_on_a_writerless_fifo_is_refused_without_blocking() {
    let result = run_with_timeout(GUARD_TIMEOUT, || edit_status_in_temp_dir("pipe", make_fifo))
        .expect("F4 on a writer-less FIFO must return, not block the event loop forever");

    let (status, opened) = result.expect("a FIFO is not a handler failure");
    assert!(
        status.starts_with("Cannot open") && status.ends_with("not a regular file"),
        "status was {status:?}"
    );
    assert!(!opened, "no editor should have been opened");
}

/// `/dev/zero` is endless and reports zero length — the exact shape the size
/// guard could not catch.
#[test]
fn f4_on_an_endless_device_node_is_refused_without_exhausting_memory() {
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

        handle_edit_builtin(&mut app)?;
        Ok::<_, io::Error>((app.status_text(), !matches!(app.overlay, Overlay::None)))
    })
    .expect("F4 on /dev/zero must return promptly instead of reading until it is killed");

    let (status, opened) = result.expect("a device node is not a handler failure");
    assert!(
        status.ends_with("not a regular file"),
        "status was {status:?}"
    );
    assert!(!opened, "no editor should have been opened");
}
