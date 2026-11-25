//! Purpose: Pure stream buffering and self-containment checks for tree copies.

use std::io;
use std::path::{Path, PathBuf};

#[cfg(not(target_os = "linux"))]
const MIN_COPY_BUFFER_BYTES: usize = 8 * 1024;
#[cfg(not(target_os = "linux"))]
const MAX_COPY_BUFFER_BYTES: usize = 256 * 1024;

/// Copy `input` to `output`, sizing a user-space buffer to `size_hint` outside
/// Linux (where `io::copy` uses kernel `copy_file_range` directly on `File`).
pub(super) fn copy_bytes<R: io::Read, W: io::Write>(
    input: &mut R,
    output: &mut W,
    size_hint: u64,
) -> io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        let _ = size_hint;
        io::copy(input, output)?;
    }
    #[cfg(not(target_os = "linux"))]
    {
        let cap = usize::try_from(size_hint)
            .unwrap_or(MAX_COPY_BUFFER_BYTES)
            .clamp(MIN_COPY_BUFFER_BYTES, MAX_COPY_BUFFER_BYTES);
        io::copy(&mut io::BufReader::with_capacity(cap, input), output)?;
    }
    Ok(())
}

/// Refuse to copy a folder into itself or into one of its children: the
/// walk would descend into the copy it is writing and duplicate the tree
/// until the path length gave out. The destination does not exist yet, so
/// its closest existing ancestor stands in for it.
pub(super) fn reject_copy_into_itself(src: &Path, dst: &Path) -> io::Result<()> {
    let (Ok(real_src), Some(real_dst)) = (src.canonicalize(), canonical_existing_ancestor(dst))
    else {
        return Ok(()); // an unreadable path fails with its own error further down
    };
    if real_dst.starts_with(&real_src) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("cannot copy a directory into itself: {}", src.display()),
        ));
    }
    Ok(())
}

fn canonical_existing_ancestor(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find_map(|ancestor| ancestor.canonicalize().ok())
}
