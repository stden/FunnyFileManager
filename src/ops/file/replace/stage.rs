//! Purpose: [`super::Replacement::Staged`] — the new entry made whole beside
//! the old one, then renamed over it.

use std::{fs, io, path::Path};

#[cfg(unix)]
use super::keeps_replaced_mode;
use super::{copies_as_file, folder_of};
use crate::copy::CopyPrompt;
use crate::utils::FailedAt;
#[cfg(unix)]
use crate::utils::FileMode;

/// Make the replacement beside `dest` and rename it over `dest` once it holds
/// everything, so a read, write or metadata failure keeps `dest`.
///
/// A file is written with no name at all where the system can make one
/// ([`crate::utils::unnamed_file_in`]) and named only once it is whole, just
/// before the rename: a copy killed half-way — a closed terminal, `kill` —
/// used to leave a `.ffm-copy-*` file in the folder for good. Elsewhere, and
/// for a link, the staged entry has that name from the start; a failure
/// FFM survives removes it, a kill does not.
pub(super) fn stage(source: &Path, dest: &Path, settings: &CopyPrompt) -> io::Result<()> {
    stage_with(
        source,
        dest,
        settings,
        crate::utils::unnamed_file_in,
        crate::utils::name_unnamed,
    )
}

pub(super) fn stage_with(
    source: &Path,
    dest: &Path,
    settings: &CopyPrompt,
    make_unnamed: impl FnOnce(&Path) -> Option<fs::File>,
    name_it: impl Fn(&fs::File, &Path) -> io::Result<()>,
) -> io::Result<()> {
    let folder = folder_of(dest);
    #[cfg(unix)]
    crate::platform::check_write_access(folder).map_err(FailedAt::for_path(folder))?;
    #[cfg(unix)]
    let kept_mode = replaced_mode(source, dest, settings);
    let staged = match unnamed_stage_with(
        source,
        folder,
        settings,
        #[cfg(unix)]
        kept_mode.as_ref(),
        make_unnamed,
        name_it,
    )? {
        Some(staged) => staged,
        None => named_stage(
            source,
            dest,
            folder,
            settings,
            #[cfg(unix)]
            kept_mode,
        )?,
    };
    finish_stage(staged, dest)
}

#[cfg(unix)]
fn replaced_mode(source: &Path, dest: &Path, settings: &CopyPrompt) -> Option<fs::Permissions> {
    fs::symlink_metadata(dest)
        .ok()
        .filter(|meta| meta.is_file() && keeps_replaced_mode(settings))
        .filter(|_| copies_as_file(source, settings))
        .map(|meta| meta.permissions())
}

pub(super) fn named_stage(
    source: &Path,
    dest: &Path,
    folder: &Path,
    settings: &CopyPrompt,
    #[cfg(unix)] kept_mode: Option<fs::Permissions>,
) -> io::Result<tempfile::NamedTempFile<()>> {
    // copy_entry claims the name exclusively and cleans up a failed
    // copy. Once it succeeds, the temporary owns cleanup for every later
    // failure. A sibling also keeps the meaning of a relative link.
    let staged = named(folder, |path| {
        crate::progress::with_destination(dest, || {
            super::super::copy_entry(source, path, settings.rules())
        })
    })?;
    #[cfg(unix)]
    if let Some(mode) = kept_mode {
        fs::set_permissions(staged.path(), mode)?;
    }
    Ok(staged)
}

fn finish_stage(mut staged: tempfile::NamedTempFile<()>, dest: &Path) -> io::Result<()> {
    // tempfile::persist clears Windows attributes. This entry was created
    // by copy_entry, so it has no temporary attribute to clear.
    prepare_windows_target(dest).map_err(FailedAt::for_path(dest))?;
    fs::rename(staged.path(), dest).map_err(FailedAt::for_path(dest))?;
    staged.disable_cleanup(true);
    Ok(())
}

#[cfg(windows)]
pub(super) fn is_windows_dir_symlink(path: &Path) -> bool {
    use std::os::windows::fs::FileTypeExt;
    fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink_dir())
}

#[cfg(not(windows))]
pub(super) const fn is_windows_dir_symlink(_path: &Path) -> bool {
    false
}

#[cfg(windows)]
#[expect(
    clippy::permissions_set_readonly_false,
    reason = "Windows: clears the read-only attribute of a destination about to be overwritten"
)]
pub(super) fn clear_windows_readonly(dest: &Path) -> io::Result<()> {
    let permissions = fs::symlink_metadata(dest)?.permissions();
    if !permissions.readonly() {
        return Err(io::Error::from(io::ErrorKind::PermissionDenied));
    }
    let mut writable = permissions;
    writable.set_readonly(false);
    fs::set_permissions(dest, writable)
}

#[cfg(not(windows))]
pub(super) fn clear_windows_readonly(_dest: &Path) -> io::Result<()> {
    Err(io::Error::from(io::ErrorKind::PermissionDenied))
}

#[cfg_attr(
    not(windows),
    expect(
        clippy::unnecessary_wraps,
        reason = "Windows removes a directory symlink here, which is fallible"
    )
)]
fn prepare_windows_target(dest: &Path) -> io::Result<()> {
    drop(clear_windows_readonly(dest));
    #[cfg(windows)]
    if is_windows_dir_symlink(dest) {
        fs::remove_dir(dest)?;
    }
    Ok(())
}

/// A free `.ffm-copy-*` name in `folder`, claimed by `claim`, and removed
/// again when the returned guard drops without `disable_cleanup`.
fn named(
    folder: &Path,
    claim: impl FnMut(&Path) -> io::Result<()>,
) -> io::Result<tempfile::NamedTempFile<()>> {
    tempfile::Builder::new()
        .prefix(".ffm-copy-")
        .make_in(folder, claim)
}

/// The copy of the regular file `source`, whole, in a file of `folder` that
/// has no name yet — or `None` when `source` is not copied as a file or the
/// system cannot make such a file, and the named staging takes over.
pub(super) fn unnamed_stage_with(
    source: &Path,
    folder: &Path,
    settings: &CopyPrompt,
    #[cfg(unix)] kept_mode: Option<&fs::Permissions>,
    make_unnamed: impl FnOnce(&Path) -> Option<fs::File>,
    name_it: impl Fn(&fs::File, &Path) -> io::Result<()>,
) -> io::Result<Option<tempfile::NamedTempFile<()>>> {
    if !copies_as_file(source, settings) {
        return Ok(None);
    }
    let Some(mut file) = make_unnamed(folder) else {
        return Ok(None);
    };
    let input = crate::utils::open_regular_file(source)?;
    #[cfg(unix)]
    let rules = {
        let mut rules = settings.rules();
        if kept_mode.is_some() {
            rules.modes = FileMode::Fresh;
        }
        rules
    };
    #[cfg(not(unix))]
    let rules = settings.rules();
    crate::utils::write_contents(&input, &mut file, rules)?;
    #[cfg(unix)]
    if let Some(mode) = kept_mode {
        file.set_permissions(mode.clone())?;
    }
    named(folder, |path| name_it(&file, path)).map(Some)
}
