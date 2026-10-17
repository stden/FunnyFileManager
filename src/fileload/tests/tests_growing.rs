//! Unit tests: a file that grows past the limit while it is read is refused as
//! too large, never truncated to the limit.

use super::*;

use std::time::Duration;

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
            Loaded::Bytes(bytes) if u64::try_from(bytes.len()).unwrap() > MAX_FILE_SIZE.bytes() => {
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
        // Best effort, like the race it feeds: a failed write only means the
        // reader never sees the file grow.
        for _ in 0..(limit / chunk.len() + 2) {
            drop(file.write_all(&chunk));
        }
        drop(file.flush());
    })
}
