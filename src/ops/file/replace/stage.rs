//! Purpose: [`super::Replacement::Staged`] — the new entry made whole beside
//! the old one, then renamed over it.

use std::{fs, io, path::Path};

use super::{copies_as_file, folder_of, keeps_replaced_mode, FailedAt};
use crate::copy::CopyPrompt;
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
    let folder = folder_of(dest);
    #[cfg(unix)]
    crate::platform::check_write_access(folder).map_err(|error| FailedAt::wrap(folder, error))?;
    let kept_mode = fs::symlink_metadata(dest)
        .ok()
        .filter(|meta| meta.is_file() && keeps_replaced_mode(settings))
        .filter(|_| copies_as_file(source, settings))
        .map(|meta| meta.permissions());
    let unnamed = unnamed_copy(source, folder, settings, kept_mode.as_ref())?;
    let mut staged = if let Some(file) = unnamed {
        named(folder, |path| crate::utils::name_unnamed(&file, path))?
    } else {
        // copy_entry claims the name exclusively and cleans up a failed
        // copy. Once it succeeds, the temporary owns cleanup for every later
        // failure. A sibling also keeps the meaning of a relative link.
        let staged = named(folder, |path| {
            super::super::copy_entry(source, path, settings.rules())
        })?;
        if let Some(mode) = kept_mode {
            fs::set_permissions(staged.path(), mode)?;
        }
        staged
    };
    // tempfile::persist clears Windows attributes. This entry was created
    // by copy_entry, so it has no temporary attribute to clear.
    fs::rename(staged.path(), dest).map_err(|error| FailedAt::wrap(dest, error))?;
    staged.disable_cleanup(true);
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
fn unnamed_copy(
    source: &Path,
    folder: &Path,
    settings: &CopyPrompt,
    kept_mode: Option<&fs::Permissions>,
) -> io::Result<Option<fs::File>> {
    if !copies_as_file(source, settings) {
        return Ok(None);
    }
    let Some(mut file) = crate::utils::unnamed_file_in(folder) else {
        return Ok(None);
    };
    let input = crate::utils::open_regular_file(source)?;
    let mut rules = settings.rules();
    if kept_mode.is_some() {
        rules.modes = FileMode::Fresh;
    }
    crate::utils::write_contents(&input, &mut file, rules)?;
    if let Some(mode) = kept_mode {
        file.set_permissions(mode.clone())?;
    }
    Ok(Some(file))
}
