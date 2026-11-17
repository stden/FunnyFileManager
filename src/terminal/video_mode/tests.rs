//! Unit tests for `settled_rows_with` height change detection and timeout fallback.

use super::*;
use std::time::Duration;

#[test]
fn settled_rows_returns_new_height_or_times_out_unchanged() {
    let changed = settled_rows_with(
        30,
        Duration::from_millis(20),
        Duration::from_millis(1),
        || Ok((80, 50)),
    );
    assert_eq!(changed, 50);

    let unchanged = settled_rows_with(
        30,
        Duration::from_millis(3),
        Duration::from_millis(1),
        || Err(io::Error::other("no tty")),
    );
    assert_eq!(unchanged, 30);
}

struct FailingWriter;

impl io::Write for FailingWriter {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::from(io::ErrorKind::BrokenPipe))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn toggle_video_mode_covers_resize_requests_and_write_failures() {
    drop(terminal_size());
    {
        let _lock = VIDEO_MODE_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        SIMULATE_NO_TTY.with(|flag| flag.set(true));
        let res = toggle_video_mode();
        SIMULATE_NO_TTY.with(|flag| flag.set(false));
        assert!(res.is_err());
    }

    assert!(toggle_video_mode_with(&mut FailingWriter, || Ok((80, 25)), Duration::ZERO).is_err());

    let expanded = toggle_video_mode_with(&mut Vec::new(), || Ok((80, 25)), Duration::ZERO)
        .expect("expand to 50");
    assert_eq!(expanded.requested_rows, 50);

    let shrunk = toggle_video_mode_with(&mut Vec::new(), || Ok((80, 40)), Duration::ZERO)
        .expect("shrink to 25");
    assert_eq!(shrunk.requested_rows, 25);
}
