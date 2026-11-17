//! Purpose: copying one regular file, the only kind of entry whose bytes a
//! transfer reads. Shared by the top-level F5/F6 transfer and the folder walk,
//! so the two cannot disagree about what they refuse.

use std::{fs, io, path::Path};

use super::keep_times::{file_times, Times};
use super::CopyRules;

mod sparse;
#[cfg(any(unix, test))]
use sparse::copy_keeping_holes;
#[cfg(unix)]
use sparse::looks_sparse;

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
    let (input, source) = open_regular_file_with_metadata(src)?;
    let shown = crate::progress::is_shown();
    if shown {
        crate::progress::sending(src, dst, 0);
    }
    // A refusal here is the destination's — a folder that takes no new file
    // — and is reported there; one on the source above is the source's.
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dst)
        .map_err(super::FailedAt::for_path(dst))?;
    let result = write_contents_reporting(&input, &source, &mut output, rules, |size| {
        if shown {
            crate::progress::sending(src, dst, size);
        }
    });
    drop(output);
    if let Err(error) = result {
        // Ours to remove: the exclusive create above is what made it. The
        // copy's own error is the one to report; a failed cleanup must not
        // replace it.
        drop(fs::remove_file(dst));
        return Err(error);
    }
    crate::progress::file_copied();
    Ok(())
}

/// Write `input`'s bytes into the new `output`, then give it its times and
/// its mode — the times first, through the handle, so that a read-only mode
/// cannot stand in their way.
pub fn write_contents(input: &fs::File, output: &mut fs::File, rules: CopyRules) -> io::Result<()> {
    let source = input.metadata()?;
    write_contents_reporting(input, &source, output, rules, |_| {})
}

fn write_contents_reporting(
    input: &fs::File,
    source: &fs::Metadata,
    output: &mut fs::File,
    rules: CopyRules,
    report_size: impl FnOnce(u64),
) -> io::Result<()> {
    // Metadata was read before the copy: reading the file moves its access time.
    report_size(source.len());
    copy_payload(source, input, output)?;
    if rules.times == Times::Source {
        output.set_times(file_times(source)?)?;
    }
    match rules.modes {
        FileMode::Source if cfg!(unix) || source.permissions().readonly() => {
            output.set_permissions(source.permissions())
        }
        FileMode::Source | FileMode::Fresh => Ok(()),
    }
}

#[cfg(unix)]
fn copy_payload(source: &fs::Metadata, input: &fs::File, output: &mut fs::File) -> io::Result<()> {
    if looks_sparse(source) {
        copy_keeping_holes(input, output)
    } else {
        copy_in_chunks(source.len(), input, output)
    }
}

#[cfg(not(unix))]
fn copy_payload(source: &fs::Metadata, input: &fs::File, output: &mut fs::File) -> io::Result<()> {
    copy_in_chunks(source.len(), input, output)
}

/// How much of a file is copied between two progress reports. Small enough
/// that a big file moves the progress bar several times a second on a slow
/// disk, large enough that the reports cost nothing next to the copy.
const PROGRESS_CHUNK: u64 = 4 * 1024 * 1024;
#[cfg(not(target_os = "linux"))]
const MIN_COPY_BUFFER_BYTES: usize = 8 * 1024;
#[cfg(not(target_os = "linux"))]
const MAX_COPY_BUFFER_BYTES: usize = 256 * 1024;

/// Copy `input` into `output` a chunk at a time, reporting each chunk to the
/// progress dialog. Each chunk is still an `io::copy`, which lets the kernel
/// copy the blocks itself (`copy_file_range`), or share them on a filesystem
/// that can: `Take<&File>` keeps that path on Linux, while a size-capped
/// 8–256 KiB buffer avoids 8 KiB fallback reads on big files and 256 KiB
/// allocations on tiny ones elsewhere.
fn copy_in_chunks(size_hint: u64, input: &fs::File, output: &mut fs::File) -> io::Result<()> {
    use std::io::Read;

    #[cfg(target_os = "linux")]
    let _ = size_hint;
    #[cfg(target_os = "linux")]
    let mut reader = input;
    #[cfg(not(target_os = "linux"))]
    let capacity = usize::try_from(size_hint)
        .unwrap_or(MAX_COPY_BUFFER_BYTES)
        .clamp(MIN_COPY_BUFFER_BYTES, MAX_COPY_BUFFER_BYTES);
    #[cfg(not(target_os = "linux"))]
    let mut reader = io::BufReader::with_capacity(capacity, input);
    loop {
        let copied = io::copy(&mut (&mut reader).take(PROGRESS_CHUNK), output)?;
        if copied == 0 {
            return Ok(());
        }
        crate::progress::copied(copied);
    }
}

/// Open `path` for reading, refusing anything that is not a regular file and
/// never waiting on a FIFO — the check lives in [`crate::fileload::open_regular`].
pub fn open_regular_file(path: &Path) -> io::Result<fs::File> {
    crate::fileload::open_regular(path, fs::OpenOptions::new().read(true))
}

/// Like [`open_regular_file`], returning the handle's already-queried metadata.
pub fn open_regular_file_with_metadata(path: &Path) -> io::Result<(fs::File, fs::Metadata)> {
    crate::fileload::open_regular_with_metadata(path, fs::OpenOptions::new().read(true))
}

#[cfg(test)]
mod tests;
