//! Purpose: Making files read-only for the tests that expect a refusal, and telling when this
//! process writes through such permissions anyway (root does).

use super::write_file_at;
use std::fs;
use std::path::Path;

pub(super) fn set_locked_state(dir: &Path, file: &Path, lock: bool) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = file;
        let mode = if lock { 0o555 } else { 0o755 };
        fs::set_permissions(dir, fs::Permissions::from_mode(mode)).unwrap();
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
        drop(crate::fsops::apply_readonly(file, lock));
    }
}

/// Whether this process writes through read-only permissions, as root does. Then nothing is
/// refused, and the refusal assertions cannot hold.
pub(super) fn writes_through_read_only(dir: &Path) -> bool {
    let probe = dir.join("probe.txt");
    write_file_at(&probe, "probe");
    set_locked_state(dir, &probe, true);
    let writable = fs::OpenOptions::new().append(true).open(&probe).is_ok();
    set_locked_state(dir, &probe, false);
    writable
}
