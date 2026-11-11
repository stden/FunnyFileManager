//! Purpose: an overwrite cut short by a kill leaves no staged copy.

use super::*;

/// The guest sweep's case: F5 "Overwrite" of a large file onto an older one,
/// and ffm killed a few seconds in (there, by closing its tmux session). The
/// staged copy was a named `.ffm-copy-*` file, and it stayed in the
/// destination folder for good. Killed outright here: the copy runs on the
/// event-loop thread, which notices a hang-up only once it is over.
#[test]
fn an_overwrite_cut_short_by_a_kill_leaves_nothing_behind() {
    let sandbox = TempDir::new().expect("sandbox");
    let work = sandbox.path().join("work");
    let out = work.join("out");
    fs::create_dir_all(&out).expect("out");
    // Holes read as zeros: 16 GiB to read, and no disk space to take.
    File::create(work.join("big.img"))
        .and_then(|file| file.set_len(16 << 30))
        .expect("big.img");
    fs::write(out.join("big.img"), b"old").expect("old big.img");
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut ffm = Ffm::start_in(sandbox, &path, &[]);

    // The other panel into `out`, the cursor on big.img (last), then F5,
    // "Already existing files: Overwrite" and Enter.
    ffm.type_keys(b"\t:cd out\r");
    ffm.type_keys(b"\t\x1b[F");
    ffm.type_keys(b"\x1b[15~");
    ffm.type_keys(b"\t\t\x1b[C\r");
    let copying = ffm.wait_for(PATIENCE, |ffm| writes_into(ffm.process.id(), &out));
    assert!(copying.is_some(), "the copy never started");

    ffm.process.kill().expect("kill ffm");
    ffm.process.wait().expect("ffm ended");

    let left: Vec<_> = fs::read_dir(&out)
        .expect("out")
        .map(|entry| entry.expect("entry").file_name())
        .collect();
    assert_eq!(left, [OsStr::new("big.img")], "nothing staged is left");
    assert_eq!(fs::read(out.join("big.img")).expect("read"), b"old");
}

/// Whether process `pid` has a file of `folder` open — a staged copy, named
/// or not — or a named staged copy already sits there.
fn writes_into(pid: u32, folder: &Path) -> bool {
    let staged_by_name = fs::read_dir(folder).is_ok_and(|mut entries| {
        entries.any(|entry| {
            entry.is_ok_and(|entry| entry.file_name().as_bytes().starts_with(b".ffm-copy-"))
        })
    });
    let open = fs::read_dir(format!("/proc/{pid}/fd")).is_ok_and(|mut handles| {
        handles.any(|handle| {
            handle
                .and_then(|handle| fs::read_link(handle.path()))
                .is_ok_and(|target| target.starts_with(folder))
        })
    });
    staged_by_name || open
}
