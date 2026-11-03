//! Purpose: Cross-platform directory link and dangling-link fixtures without elevation on Windows.

use std::io;
use std::path::Path;

/// Create a directory symbolic link (or an NTFS junction when Windows symlink
/// privileges are unavailable) at `link` pointing to `target`.
pub fn make_dir_link(target: &Path, link: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(windows)]
    {
        if std::os::windows::fs::symlink_dir(target, link).is_ok() {
            return Ok(());
        }
        let output = std::process::Command::new("cmd.exe")
            .args(["/D", "/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()?;
        if output.status.success() {
            Ok(())
        } else {
            Err(io::Error::other(
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ))
        }
    }
}

/// Create a dangling link at `link` whose target `missing_target` does not exist.
pub fn make_dangling_link(missing_target: &Path, link: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(missing_target, link)
    }
    #[cfg(windows)]
    {
        let created_temp = !missing_target.exists();
        if created_temp {
            std::fs::create_dir_all(missing_target)?;
        }
        let linked = make_dir_link(missing_target, link);
        if created_temp {
            drop(std::fs::remove_dir(missing_target));
        }
        linked
    }
}
