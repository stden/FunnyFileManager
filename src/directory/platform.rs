//! Platform-specific entry facts: allocation, executability, and hidden flags.

use std::{fs, path::Path};

use crate::values::entry::FileSize;

/// `st_blocks` counts 512-byte units whatever the filesystem's block size;
/// elsewhere the length is the best available answer.
#[cfg(unix)]
pub(super) fn allocated_bytes(metadata: &fs::Metadata) -> FileSize {
    use std::os::unix::fs::MetadataExt;

    FileSize::from_bytes(metadata.blocks().saturating_mul(512))
}

#[cfg(not(unix))]
pub(super) fn allocated_bytes(metadata: &fs::Metadata) -> FileSize {
    FileSize::from_bytes(metadata.len())
}

/// Unix execute bits, script extensions (`*.cmd`, `*.sh`, `*.py`), and Windows
/// runnable extensions mark files executable in panels.
#[cfg(unix)]
pub(super) fn is_executable(name: &str, metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    const SCRIPTS: [&str; 3] = ["cmd", "sh", "py"];
    metadata.is_file()
        && (metadata.permissions().mode() & 0o111 != 0 || has_extension(name, &SCRIPTS))
}

#[cfg(not(unix))]
pub(super) fn is_executable(name: &str, metadata: &fs::Metadata) -> bool {
    const RUNNABLE: [&str; 8] = ["exe", "com", "bat", "cmd", "ps1", "msi", "sh", "py"];
    metadata.is_file() && has_extension(name, &RUNNABLE)
}

fn has_extension(name: &str, exts: &[&str]) -> bool {
    let Some(ext) = Path::new(name).extension().and_then(|e| e.to_str()) else {
        return false;
    };
    exts.iter().any(|c| ext.eq_ignore_ascii_case(c))
}

/// Windows stores a hidden attribute; a Unix leading dot is just a name.
#[cfg(windows)]
pub(super) fn is_hidden(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0
}

#[cfg(not(windows))]
pub(super) const fn is_hidden(_metadata: &fs::Metadata) -> bool {
    false
}
