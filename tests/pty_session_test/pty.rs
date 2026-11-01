//! Purpose: a pseudo-terminal pair for exercising the real FFM process.

use super::*;

/// The nonblocking far end the test holds, and the terminal FFM runs on.
#[expect(
    clippy::expect_used,
    reason = "test fixture: an error from the pty fails the test driving it"
)]
pub fn open_pty() -> (File, File) {
    let far_end =
        openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY | OpenptFlags::CLOEXEC).expect("open a pty");
    grantpt(&far_end).expect("grant the pty");
    unlockpt(&far_end).expect("unlock the pty");
    let size = Winsize {
        ws_row: 30,
        ws_col: 100,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    tcsetwinsize(&far_end, size).expect("size the pty");
    rustix::io::ioctl_fionbio(&far_end, true).expect("nonblocking pty");
    let name = ptsname(&far_end, Vec::new()).expect("name the pty");
    let device = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY)
        .open(OsStr::from_bytes(name.as_bytes()))
        .expect("open the terminal device");
    (File::from(far_end), device)
}

#[expect(
    clippy::expect_used,
    reason = "test fixture: sandbox directories must exist"
)]
pub fn sandbox_directories(sandbox: &TempDir) -> (PathBuf, PathBuf) {
    let home = sandbox.path().join("home");
    let tmp = sandbox.path().join("tmp");
    for directory in [&home, &tmp, &sandbox.path().join("work")] {
        fs::create_dir_all(directory).expect("sandbox directory");
    }
    (home, tmp)
}
