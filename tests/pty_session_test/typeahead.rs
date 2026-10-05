//! Purpose: keys typed while a long copy holds the screen never run once it
//! is over — in the real binary, through crossterm's own queue.

use super::*;

/// The guest sweep's data loss: F5 Enter on a large file, then Up F8 Enter
/// typed at the frozen screen. Once the copy was over the keys ran blind and
/// deleted a-keep.txt.
#[test]
fn keys_typed_during_a_long_copy_are_thrown_away() {
    let sandbox = TempDir::new().expect("sandbox");
    let work = sandbox.path().join("work");
    fs::create_dir_all(work.join("out")).expect("out");
    fs::write(work.join("a-keep.txt"), b"precious").expect("a-keep.txt");
    // Holes read as zeros: long enough to read to freeze the screen, and no
    // disk space to take.
    let size = 256 << 20;
    File::create(work.join("big.img"))
        .and_then(|file| file.set_len(size))
        .expect("big.img");
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut ffm = Ffm::start_in(sandbox, &path, &[]);

    // The other panel into `out`, the cursor on big.img (last), then — in
    // one burst, as a fast typist would — F5, Enter, and Up F8 Enter, which
    // land in the queue while the copy runs.
    ffm.type_keys(b"\t:cd out\r\t\x1b[F");
    ffm.type_keys(b"\x1b[15~\r\x1b[A\x1b[19~\r");
    let copied = ffm.wait_for(PATIENCE, |ffm| contains(&ffm.screen, b"Copied to"));
    assert!(copied.is_some(), "the copy never finished");

    // Typed after the copy answered: this one runs, so once it has, so has
    // anything the loop did not throw away.
    ffm.type_keys(b":mkdir probe\r");
    let probed = ffm.wait_for(PATIENCE, |ffm| ffm.work().join("probe").is_dir());
    assert!(probed.is_some(), "a key typed after the copy was lost");
    assert_eq!(
        fs::read(ffm.work().join("a-keep.txt")).expect("kept"),
        b"precious"
    );
    let copy = fs::metadata(ffm.work().join("out/big.img")).expect("copied");
    assert_eq!(copy.len(), size);
}
