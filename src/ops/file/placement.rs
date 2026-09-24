//! Purpose: Put one entry at its dialog destination: settle a clash with what
//! is already there, copy or move it, then apply the dialog's options.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use super::entry::copy_entry;
use super::{merge, rename_or_fail, replace};
use crate::copy::Purpose;
use crate::utils::unique_destination;

/// Where one entry of a dialog transfer ended up.
pub(super) enum Placed {
    /// Under the name it was sent to.
    AsSent(PathBuf),
    /// Beside something already there, under a free name.
    Beside(PathBuf),
    /// Left alone, as the "already existing files" choice said.
    Skipped,
}

/// `dest` is taken: do what the "already existing files" choice says.
///
/// Skipping is a normal outcome, not a failure — reporting it as an error
/// would put a red count on a deliberate choice. Overwriting never removes
/// what the transfer does not bring; see [`merge`].
pub(super) fn place_over_existing(
    source: &Path,
    dest: &Path,
    settings: &crate::copy::CopyPrompt,
) -> io::Result<Placed> {
    use crate::copy::ExistingFiles;
    use merge::Shape;

    let shapes = (
        Shape::of_source(source, settings),
        Shape::of_existing(dest)?,
    );
    let folders = shapes == (Shape::Folder, Shape::Folder);
    match settings.existing {
        ExistingFiles::Skip => return Ok(Placed::Skipped),
        // Two folders merge, and "only newer" is decided file by file inside.
        ExistingFiles::OnlyNewer if !folders && !is_newer(source, dest) => {
            return Ok(Placed::Skipped);
        }
        _ => {}
    }
    reject_transfer_onto_itself(source, dest)?;
    if settings.existing == ExistingFiles::Ask {
        // Not yet a per-file prompt. Until it is, the non-destructive reading
        // of "ask" is taken: nothing already there is touched, and the new one
        // lands beside it under a free name.
        let parent = dest.parent().unwrap_or(dest);
        let beside = unique_destination(parent, dest.file_name().unwrap_or_default())?;
        place(source, &beside, settings)?;
        return Ok(Placed::Beside(beside));
    }
    merge::reject_mixed_shapes(shapes.0, shapes.1)?;
    if folders {
        merge::reject_merge_into_holder(source, dest)?;
        merge::merge_into(source, dest, settings)?;
    } else {
        replace::entry(source, dest, settings)?;
    }
    Ok(Placed::AsSent(dest.to_path_buf()))
}

/// Refuse a transfer whose destination *is* its source.
///
/// Both panels start in the same directory, so the dialog's default
/// destination is often the source's own. Far refuses a copy or a move onto
/// itself whatever the "already existing files" choice: "Overwrite" would
/// remove the file and then copy from what is no longer there, and "Ask"
/// would rename a moved file to `name_1` and call that a move.
pub(super) fn reject_transfer_onto_itself(source: &Path, dest: &Path) -> io::Result<()> {
    let same = source == dest
        || replace::are_hard_links(source, dest)
        || matches!(
            (source.canonicalize(), dest.canonicalize()),
            (Ok(real_source), Ok(real_dest)) if real_source == real_dest
        );
    if same {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cannot copy or move onto itself",
        ));
    }
    Ok(())
}

/// Put `source` at `dest`, copying or moving as the dialog said.
///
/// A move is tried as a rename first — within one filesystem that is the whole
/// operation and it is atomic. Across filesystems `rename` fails, and the only
/// thing left is to copy and then remove, which is what every mover does.
pub(super) fn place(
    source: &Path,
    dest: &Path,
    settings: &crate::copy::CopyPrompt,
) -> io::Result<()> {
    if settings.purpose == Purpose::Move && rename_or_fail(source, dest).is_ok() {
        return Ok(());
    }
    copy_entry(source, dest, settings.rules())?;
    finish(source, dest, settings)?;
    if settings.purpose == Purpose::Move {
        remove_existing(source)?;
    }
    Ok(())
}

/// Whether `source` was modified after `dest`, for the "only newer" policy.
///
/// Unreadable times mean "copy it": refusing to act because a timestamp could
/// not be read would silently skip the file the user asked about.
pub(super) fn is_newer(source: &Path, dest: &Path) -> bool {
    let time = |path: &Path| fs::metadata(path).and_then(|meta| meta.modified()).ok();
    match (time(source), time(dest)) {
        (Some(from), Some(to)) => from > to,
        _ => true,
    }
}

/// Remove whatever is at `dest` so a copy can take its place.
pub(super) fn remove_existing(dest: &Path) -> io::Result<()> {
    if fs::symlink_metadata(dest)?.is_dir() {
        fs::remove_dir_all(dest)
    } else {
        fs::remove_file(dest)
    }
}

/// Apply the dialog's post-copy options to what was just written.
pub(super) fn finish(
    source: &Path,
    dest: &Path,
    settings: &crate::copy::CopyPrompt,
) -> io::Result<()> {
    if settings.timestamps {
        let modified = fs::metadata(source)?.modified()?;
        crate::ops::timestamps::open(dest)?.set_modified(modified)?;
    }
    apply_rights(source, dest, settings.rights)
}

/// Restate the source's permission bits on `dest` when the dialog says Copy.
///
/// Every file was already given its mode as it was written (see
/// [`crate::copy::AccessRights::file_mode`]). What is left is a copied
/// folder, which is created afresh and can only take a read-only source's
/// mode once its contents are in. Default and Inherit leave it as created:
/// the kernel applied the umask then, and a `chmod` now would skip it.
#[cfg(unix)]
fn apply_rights(source: &Path, dest: &Path, rights: crate::copy::AccessRights) -> io::Result<()> {
    if rights != crate::copy::AccessRights::Copy {
        return Ok(());
    }
    fs::set_permissions(dest, fs::metadata(source)?.permissions())
}

#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "shares the Unix signature, where the operation can fail"
)]
fn apply_rights(
    _source: &Path,
    _dest: &Path,
    _rights: crate::copy::AccessRights,
) -> io::Result<()> {
    // Windows has no mode bits to carry; the ACL comes from the destination
    // directory whatever the dialog says.
    Ok(())
}
