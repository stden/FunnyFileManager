//! Purpose: Sparse zero-run hole preservation during Unix file copies (`cp --sparse=auto`).

#[cfg(any(unix, test))]
use std::{fs, io};

/// Whether every byte of `block` is zero. Folds instead of `all`, which stops
/// at the first non-zero byte and so cannot be vectorised: this is the loop a
/// multi-gigabyte image spends its time in.
#[cfg(any(unix, test))]
fn is_all_zeros(block: &[u8]) -> bool {
    block.iter().fold(0, |any, &byte| any | byte) == 0
}

/// Size in bytes of the 512-byte unit reported by `MetadataExt::blocks`.
#[cfg(unix)]
const POSIX_BLOCK_BYTES: u64 = 512;

/// How much is read at a time when looking for holes. A run of zeros shorter
/// than this, or not aligned to it, is written out rather than skipped.
#[cfg(any(unix, test))]
const HOLE_BLOCK_BYTES: usize = 64 * 1024;

/// Whether the filesystem holds fewer bytes for the file than it is long —
/// `cp`'s test for holes worth keeping. A kernel-side copy of such a file
/// writes every hole out as zeros: a 5 GiB disk image that allocates nothing
/// would take 5 GiB to copy.
#[cfg(unix)]
pub(super) fn looks_sparse(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    metadata.blocks().saturating_mul(POSIX_BLOCK_BYTES) < metadata.len()
}

/// Copy block by block, seeking over each block of zeros instead of writing
/// it, so the copy has a hole wherever the source read as one
/// (`cp --sparse=auto`).
///
/// Driven by what the reads return, not by the length `stat` reported: a
/// pseudo-file that claims 4 KiB and holds 50 bytes is copied as 50 bytes.
#[cfg(any(unix, test))]
pub(super) fn copy_keeping_holes(input: &fs::File, output: &mut fs::File) -> io::Result<()> {
    use std::io::{Read, Seek, SeekFrom, Write};

    let chunk_limit = u64::try_from(HOLE_BLOCK_BYTES).unwrap_or(u64::MAX);
    let mut block = Vec::with_capacity(HOLE_BLOCK_BYTES);
    let mut stream_offset = 0_u64;
    loop {
        block.clear();
        let bytes_read = input.take(chunk_limit).read_to_end(&mut block)?;
        if bytes_read == 0 {
            break;
        }
        let step = u64::try_from(bytes_read).unwrap_or(u64::MAX);
        stream_offset = stream_offset.saturating_add(step);
        crate::progress::copied(step);
        if is_all_zeros(&block) {
            // A hole: advance the writer past the zero run without allocating blocks.
            output.seek(SeekFrom::Start(stream_offset))?;
        } else {
            output.write_all(&block)?;
        }
    }
    // Nothing is written for a trailing hole; the length says where it ends.
    output.set_len(stream_offset)
}
