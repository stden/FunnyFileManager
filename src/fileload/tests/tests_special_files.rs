//! Unit tests: what is not a regular file — an endless device node and a FIFO
//! with no writer — is refused promptly, and the read is bounded regardless.

// Only the endless-stream test needs the raw bounded read, and /dev/zero is
// Unix-only.
use super::super::read_bounded;
use super::*;

use std::time::Duration;

/// Longest a guard may take. Generous enough that a loaded machine will not
/// trip it, short enough that a blocking regression is caught in seconds.
/// Only the FIFO and device-node tests are timeout-guarded, and both are Unix.
const GUARD_TIMEOUT: Duration = Duration::from_secs(5);

/// /dev/zero reports `len() == 0`, so the old size check waved it through and
/// the unbounded `fs::read` grew a Vec until the OOM killer fired.
#[cfg(unix)]
#[test]
fn refuses_an_endless_device_node() {
    let zero = std::path::Path::new("/dev/zero");
    if !zero.exists() {
        return;
    }
    let outcome = crate::test_support::run_with_timeout(GUARD_TIMEOUT, || {
        read_for_text_ui(std::path::Path::new("/dev/zero")).map(|loaded| match loaded {
            Loaded::Bytes(bytes) => format!("read {} bytes", bytes.len()),
            Loaded::NotRegular => "not-regular".to_owned(),
            Loaded::TooLarge(size) => format!("too large: {size}"),
        })
    })
    .expect("reading /dev/zero must return promptly, not consume memory until it is killed");

    assert_eq!(
        outcome.expect("/dev/zero is readable"),
        "not-regular",
        "a device node is not something a text UI may slurp"
    );
}

/// The second half of the guard: `/dev/zero` is a *readable* endless stream, so
/// only the `take` bound ever ends the read. Without it this allocates until
/// the OOM killer fires.
#[cfg(unix)]
#[test]
fn the_read_is_bounded_even_for_an_endless_file() {
    let zero = std::path::Path::new("/dev/zero");
    if File::open(zero).is_err() {
        return;
    }

    let read = crate::test_support::run_with_timeout(GUARD_TIMEOUT, || {
        read_bounded(File::open("/dev/zero")?, MAX_FILE_SIZE.bytes()).map(|bytes| bytes.len())
    })
    .expect("a bounded read of /dev/zero must finish promptly");

    assert_eq!(
        u64::try_from(read.expect("/dev/zero is readable")).expect("fits"),
        MAX_FILE_SIZE.bytes() + 1,
        "the read must stop one byte past the limit, not at EOF"
    );
}

/// A FIFO with no writer blocks inside `File::open` forever. The file-type
/// check has to happen *before* the open, which is what this pins.
#[cfg(unix)]
#[test]
fn refuses_a_writerless_fifo_without_blocking() -> io::Result<()> {
    let temp = TempDir::new()?;
    let fifo = temp.path().join("pipe");
    crate::test_support::make_fifo(&fifo)?;

    let outcome = crate::test_support::run_with_timeout(GUARD_TIMEOUT, move || {
        read_for_text_ui(&fifo).map(|loaded| matches!(loaded, Loaded::NotRegular))
    })
    .expect("opening a writer-less FIFO must not block the caller");

    assert!(
        outcome.expect("a FIFO's metadata is readable"),
        "a FIFO must be refused as not-regular"
    );
    // `temp` is dropped only after the guard returned, proving nothing still
    // holds the pipe open.
    Ok(())
}
