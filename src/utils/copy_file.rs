//! Purpose: copying one regular file, the only kind of entry whose bytes a
//! transfer reads. Shared by the top-level F5/F6 transfer and the folder walk,
//! so the two cannot disagree about what they refuse.

use std::{fs, io, path::Path};

use super::keep_times::{file_times, Times};
use super::CopyRules;

/// Which permission bits a copied file or folder ends up with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMode {
    /// The source's, as `fs::copy` and `cp` leave them.
    Source,
    /// What any file newly created in that folder gets. The kernel works that
    /// out from the umask (or the folder's default ACL) when the file is
    /// created, and only then — a `chmod` afterwards skips the umask — so the
    /// copy is simply left as it was created.
    Fresh,
}

/// Copy the regular file `src` to `dst`, which must not exist yet.
///
/// What `fs::copy` does, with two differences a file manager needs: the
/// destination is claimed with an exclusive create, so a file that appeared
/// there is never truncated (the caller picks another name on
/// [`io::ErrorKind::AlreadyExists`]); and a copy that fails half-way is
/// removed, because under its final name it would pass for a complete one.
pub fn copy_file(src: &Path, dst: &Path, rules: CopyRules) -> io::Result<()> {
    // A FIFO would wait for a writer on the event loop, and a device node
    // copy until the disk filled: only a regular file is opened.
    let input = open_regular_file(src)?;
    // A refusal here is the destination's — a folder that takes no new file
    // — and is reported there; one on the source above is the source's.
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dst)
        .map_err(|error| super::FailedAt::wrap(dst, error))?;
    let result = write_contents(&input, &mut output, rules);
    if result.is_err() {
        // Ours to remove: the exclusive create above is what made it. The
        // copy's own error is the one to report; a failed cleanup must not
        // replace it.
        drop(fs::remove_file(dst));
    }
    result
}

/// Write `input`'s bytes into the new `output`, then give it its times and
/// its mode — the times first, through the handle, so that a read-only mode
/// cannot stand in their way.
pub fn write_contents(input: &fs::File, output: &mut fs::File, rules: CopyRules) -> io::Result<()> {
    // Read before the copy: reading the file moves its access time.
    let source = input.metadata()?;
    if looks_sparse(&source) {
        copy_keeping_holes(input, output)?;
    } else {
        // Lets the kernel copy the blocks itself (`copy_file_range`), or
        // share them on a filesystem that can.
        io::copy(&mut &*input, output)?;
    }
    if rules.times == Times::Source {
        output.set_times(file_times(&source)?)?;
    }
    match rules.modes {
        FileMode::Source => output.set_permissions(source.permissions()),
        FileMode::Fresh => Ok(()),
    }
}

/// Whether every byte of `block` is zero. Folds instead of `all`, which stops
/// at the first non-zero byte and so cannot be vectorised: this is the loop a
/// multi-gigabyte image spends its time in.
fn is_all_zeros(block: &[u8]) -> bool {
    block.iter().fold(0, |any, &byte| any | byte) == 0
}

/// How much is read at a time when looking for holes. A run of zeros shorter
/// than this, or not aligned to it, is written out rather than skipped.
const HOLE_BLOCK: u64 = 64 * 1024;

/// Whether the filesystem holds fewer bytes for the file than it is long —
/// `cp`'s test for holes worth keeping. A kernel-side copy of such a file
/// writes every hole out as zeros: a 5 GiB disk image that allocates nothing
/// would take 5 GiB to copy.
#[cfg(unix)]
fn looks_sparse(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    metadata.blocks().saturating_mul(512) < metadata.len()
}

/// A Windows file only has holes once it is flagged sparse, which a copy made
/// here never is; seeking over zeros would gain nothing there.
#[cfg(not(unix))]
const fn looks_sparse(_metadata: &fs::Metadata) -> bool {
    false
}

/// Copy block by block, seeking over each block of zeros instead of writing
/// it, so the copy has a hole wherever the source read as one
/// (`cp --sparse=auto`).
///
/// Driven by what the reads return, not by the length `stat` reported: a
/// pseudo-file that claims 4 KiB and holds 50 bytes is copied as 50 bytes.
fn copy_keeping_holes(mut input: &fs::File, output: &mut fs::File) -> io::Result<()> {
    use std::io::{Read, Seek, SeekFrom, Write};

    let mut block = Vec::new();
    loop {
        block.clear();
        if input.take(HOLE_BLOCK).read_to_end(&mut block)? == 0 {
            break;
        }
        if is_all_zeros(&block) {
            // A hole: move the writer to where the reader now is.
            output.seek(SeekFrom::Start(input.stream_position()?))?;
        } else {
            output.write_all(&block)?;
        }
    }
    // Nothing is written for a trailing hole; the length says where it ends.
    output.set_len(input.stream_position()?)
}

/// Open `path` for reading, refusing anything that is not a regular file and
/// never waiting on a FIFO — the check lives in [`crate::fileload::open_regular`].
pub fn open_regular_file(path: &Path) -> io::Result<fs::File> {
    crate::fileload::open_regular(path, fs::OpenOptions::new().read(true))
}
