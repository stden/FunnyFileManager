//! Purpose: Reports a filesystem's size and kind for the panel footer and the
//! Alt+F1 / Alt+F2 drive dialog.
//!
//! std has no API for either, so each platform's own call is wrapped here and
//! nowhere else — the UI asks for an `Option<Capacity>` and never sees the FFI.

use std::path::Path;

/// How big a filesystem is and how much of it is still writable.
///
/// Both come from one platform call: the drive dialog shows them side by side,
/// and asking twice would let the two halves disagree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capacity {
    /// Total bytes on the filesystem.
    pub total: u64,
    /// Bytes an unprivileged process may still write.
    pub available: u64,
}

/// What kind of device a drive is, as Far names it beside the letter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DriveKind {
    /// A hard disk or SSD.
    Fixed,
    /// A stick, a card, a floppy.
    Removable,
    /// A mapped share.
    Network,
    /// Optical media.
    CdRom,
    /// A memory-backed filesystem.
    RamDisk,
    /// The platform would not say.
    #[default]
    Unknown,
}

impl DriveKind {
    /// The word Far writes after the drive letter.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Removable => "removable",
            Self::Network => "network",
            Self::CdRom => "CD-ROM",
            Self::RamDisk => "RAM disk",
            // Nothing is better than a guess: the column is simply blank.
            Self::Unknown => "",
        }
    }
}

/// Bytes an unprivileged process may still write to the filesystem holding
/// `path`, or `None` when the platform call fails.
///
/// `None` rather than `0` on failure: an unmounted drive and a full one are
/// different facts, and the footer shows nothing for the first rather than
/// claiming the disk is out of space.
#[must_use]
pub fn available_bytes(path: &Path) -> Option<u64> {
    capacity(path).map(|space| space.available)
}

/// The size of the filesystem holding `path`, or `None` when the platform call
/// fails — an empty drive bay, a disconnected share.
#[cfg(unix)]
#[must_use]
pub fn capacity(path: &Path) -> Option<Capacity> {
    // rustix widens every statvfs field to u64 whatever the Unix, and a path
    // with an interior NUL fails like any other unanswerable path.
    let stats = rustix::fs::statvfs(path).ok()?;
    Some(Capacity {
        total: stats.f_blocks.checked_mul(stats.f_frsize)?,
        available: stats.f_bavail.checked_mul(stats.f_frsize)?,
    })
}

#[cfg(windows)]
#[must_use]
#[allow(unsafe_code)]
pub fn capacity(path: &Path) -> Option<Capacity> {
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let wide = wide_path(path);
    let mut available: u64 = 0;
    let mut total: u64 = 0;
    // SAFETY: `wide` is NUL-terminated and outlives the call; both output
    // pointers address owned `u64`s, and the API documents the third as
    // optional, ignoring it when null.
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &raw mut available,
            &raw mut total,
            std::ptr::null_mut(),
        )
    };
    (ok != 0).then_some(Capacity { total, available })
}

#[cfg(not(any(unix, windows)))]
#[must_use]
pub fn capacity(_path: &Path) -> Option<Capacity> {
    None
}

/// What kind of device `path` sits on.
///
/// Windows answers directly. Unix has no equivalent question — a mount is a
/// mount — so every filesystem reads as fixed rather than inventing a
/// distinction the kernel does not draw.
#[cfg(windows)]
#[must_use]
#[allow(unsafe_code)]
pub fn drive_kind(path: &Path) -> DriveKind {
    use windows_sys::Win32::{
        Storage::FileSystem::GetDriveTypeW,
        System::WindowsProgramming::{
            DRIVE_CDROM, DRIVE_FIXED, DRIVE_RAMDISK, DRIVE_REMOTE, DRIVE_REMOVABLE,
        },
    };

    let wide = wide_path(path);
    // SAFETY: `wide` is NUL-terminated and outlives the call, which only reads
    // it. The function has no failure mode beyond the codes matched below.
    let kind = unsafe { GetDriveTypeW(wide.as_ptr()) };
    match kind {
        DRIVE_FIXED => DriveKind::Fixed,
        DRIVE_REMOVABLE => DriveKind::Removable,
        DRIVE_REMOTE => DriveKind::Network,
        DRIVE_CDROM => DriveKind::CdRom,
        DRIVE_RAMDISK => DriveKind::RamDisk,
        _ => DriveKind::Unknown,
    }
}

#[cfg(not(windows))]
#[must_use]
pub fn drive_kind(_path: &Path) -> DriveKind {
    DriveKind::Fixed
}

/// A NUL-terminated UTF-16 copy of `path`, which is what every Win32 `*W` call
/// wants and what both callers above would otherwise build separately.
#[cfg(windows)]
fn wide_path(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    wide
}

#[cfg(test)]
mod tests;
