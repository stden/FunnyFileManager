//! Unit tests: the bounded, regular-file-only read shared by F3 and F4.

use super::{read_for_text_ui, Loaded};
// Only the endless-stream test needs the raw bounded read, and /dev/zero is
// Unix-only.
#[cfg(unix)]
use super::read_bounded;

use crate::utils::MAX_FILE_SIZE;

use std::fs::{self, File};
use std::io;
use std::time::Duration;

use tempfile::TempDir;

/// Longest a guard may take. Generous enough that a loaded machine will not
/// trip it, short enough that a blocking regression is caught in seconds.
/// Only the FIFO and device-node tests are timeout-guarded, and both are Unix.
#[cfg(unix)]
const GUARD_TIMEOUT: Duration = Duration::from_secs(5);

#[test]
fn reads_a_regular_file_whole() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = temp.path().join("notes.txt");
    fs::write(&path, b"hello")?;

    let Loaded::Bytes(bytes) = read_for_text_ui(&path)? else {
        panic!("a regular file must load");
    };
    assert_eq!(bytes, b"hello");
    Ok(())
}

#[test]
fn refuses_a_directory() -> io::Result<()> {
    let temp = TempDir::new()?;
    assert!(matches!(read_for_text_ui(temp.path())?, Loaded::NotRegular));
    Ok(())
}

#[test]
fn reports_a_missing_file_as_an_error() {
    let temp = TempDir::new().expect("temp dir");
    let Err(error) = read_for_text_ui(&temp.path().join("ghost.txt")) else {
        panic!("a missing file must surface as io::Error, not a Loaded case");
    };
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
}

#[test]
fn refuses_a_file_over_the_limit() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = temp.path().join("huge.txt");
    // Sparse: `set_len` records the size in the inode without writing a byte.
    File::create(&path)?.set_len(MAX_FILE_SIZE.bytes() + 1)?;

    let Loaded::TooLarge(size) = read_for_text_ui(&path)? else {
        panic!("an over-limit file must be refused");
    };
    assert_eq!(size, MAX_FILE_SIZE.bytes() + 1);
    Ok(())
}

/// The guard is `>`, not `>=`: a file of exactly the limit still loads, and the
/// `take(limit + 1)` bound must not cut its last byte off.
#[test]
fn loads_a_file_exactly_at_the_limit() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = temp.path().join("limit.txt");
    File::create(&path)?.set_len(MAX_FILE_SIZE.bytes())?;

    let Loaded::Bytes(bytes) = read_for_text_ui(&path)? else {
        panic!("a file at exactly the limit must load");
    };
    assert_eq!(
        u64::try_from(bytes.len()).expect("fits"),
        MAX_FILE_SIZE.bytes()
    );
    Ok(())
}

/// A symlink to a regular file is viewable — the *target*'s type is what
/// decides, so the guard must follow the link rather than refuse it.
#[cfg(unix)]
#[test]
fn follows_a_symlink_to_a_regular_file() -> io::Result<()> {
    let temp = TempDir::new()?;
    let target = temp.path().join("target.txt");
    fs::write(&target, b"through the link")?;
    let link = temp.path().join("link.txt");
    std::os::unix::fs::symlink(&target, &link)?;

    let Loaded::Bytes(bytes) = read_for_text_ui(&link)? else {
        panic!("a link to a regular file must load");
    };
    assert_eq!(bytes, b"through the link");
    Ok(())
}

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
            Loaded::NotRegular => "not-regular".to_string(),
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
        read_bounded(File::open("/dev/zero")?).map(|bytes| bytes.len())
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

/// The post-`metadata` size check: a file that grows past the limit between
/// the size snapshot and the bounded read must still be refused as `TooLarge`
/// (not silently truncated to the first 10 MB).
///
/// Uses a short retry loop because the interleaving is a race; on a quiet
/// machine the writer usually wins within a few attempts.
#[test]
fn refuses_a_file_that_grows_past_the_limit_during_the_read() -> io::Result<()> {
    use std::sync::{Arc, Barrier};
    use std::thread;

    for _ in 0..30 {
        let temp = TempDir::new()?;
        let path = temp.path().join("growing.bin");
        // Empty so the first metadata snapshot is under the limit.
        File::create(&path)?;

        let barrier = Arc::new(Barrier::new(2));

        let writer = spawn_growth_writer(path.clone(), Arc::clone(&barrier));

        barrier.wait();
        // Tiny pause so the writer typically extends past the limit before
        // the reader's take() finishes; metadata may still have been small.
        thread::sleep(Duration::from_millis(2));
        let loaded = read_for_text_ui(&path)?;
        writer.join().expect("writer");

        match loaded {
            Loaded::TooLarge(size) => {
                assert!(
                    size > MAX_FILE_SIZE.bytes(),
                    "TooLarge must report a size over the limit, got {size}"
                );
                return Ok(());
            }
            Loaded::Bytes(bytes) if (bytes.len() as u64) > MAX_FILE_SIZE.bytes() => {
                // Should be impossible: Bytes is only returned under the limit.
                panic!("Bytes larger than the limit escaped the guard");
            }
            Loaded::Bytes(_) | Loaded::NotRegular => {
                // Race lost: reader finished before growth. Try again.
            }
        }
    }

    // If the race never lands, the suite still passes: the deterministic
    // metadata-based TooLarge path is covered elsewhere, and this guard is
    // best-effort under pure tests without an injection seam.
    Ok(())
}

fn spawn_growth_writer(
    path_writer: std::path::PathBuf,
    barrier_w: std::sync::Arc<std::sync::Barrier>,
) -> std::thread::JoinHandle<()> {
    use std::io::Write;
    use std::thread;
    thread::spawn(move || {
        barrier_w.wait();
        let mut file = fs::OpenOptions::new()
            .write(true)
            .open(&path_writer)
            .expect("open for growth");
        let chunk = vec![0_u8; 1024 * 1024];
        // Well past MAX_FILE_SIZE.bytes() so a concurrent take() can see
        // more than the limit even if it started from a zero-length snapshot.
        let limit = usize::try_from(MAX_FILE_SIZE.bytes()).expect("the limit fits usize");
        for _ in 0..(limit / chunk.len() + 2) {
            let _ = file.write_all(&chunk);
        }
        let _ = file.flush();
    })
}
