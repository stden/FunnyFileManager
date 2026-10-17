//! Purpose: Put one entry at its dialog destination: settle a clash with what
//! is already there, copy or move it, then apply the dialog's options.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use super::entry::copy_entry;
use super::{merge, remove_moved, rename_within, replace, Renamed};
use crate::copy::Purpose;
use crate::utils::unique_destination;

/// Where one entry of a dialog transfer ended up.
pub(super) enum Placed {
    /// Under the name it was sent to.
    AsSent(PathBuf),
    /// Into the folder already at that name, with `skipped` entries inside
    /// left alone by the "already existing files" choice.
    Merged { into: PathBuf, skipped: usize },
    /// Beside something already there, under a free name.
    Beside(PathBuf),
    /// Left alone, as the "already existing files" choice said.
    Skipped,
}

impl Placed {
    /// How many entries the choice left alone on the way, so the status line
    /// can say so however deep in a merge they were.
    pub(super) const fn skipped(&self) -> usize {
        match self {
            Self::Skipped => 1,
            Self::Merged { skipped, .. } => *skipped,
            Self::AsSent(_) | Self::Beside(_) => 0,
        }
    }
}

/// `dest` is taken: do what the "already existing files" choice says.
///
/// Skipping is a normal outcome, not a failure — reporting it as an error
/// would put a red count on a deliberate choice. Every choice but "Ask"
/// merges a folder into a folder and decides file by file inside, as Far and
/// mc do: "Skip" leaves the files already there, not the whole folder.
/// Overwriting never removes what the transfer does not bring; see [`merge`].
pub(super) fn place_over_existing(
    source: &Path,
    dest: &Path,
    settings: &crate::copy::CopyPrompt,
) -> io::Result<Placed> {
    use crate::copy::ExistingFiles;
    use merge::Shape;

    if settings.purpose == Purpose::Move && is_case_only_rename(source, dest) {
        return place(source, dest, settings).map(|()| Placed::AsSent(dest.to_path_buf()));
    }
    let shapes = (
        Shape::of_source(source, settings),
        Shape::of_existing(dest)?,
    );
    let folders = shapes == (Shape::Folder, Shape::Folder);
    if !folders
        && (settings.existing == ExistingFiles::Skip
            || (settings.existing == ExistingFiles::OnlyNewer && !is_newer(source, dest)))
    {
        return Ok(Placed::Skipped);
    }
    reject_transfer_onto_itself(source, dest)?;
    if settings.existing == ExistingFiles::Ask {
        return place_beside(source, dest, settings);
    }
    merge::reject_mixed_shapes(shapes.0, shapes.1)?;
    if folders {
        merge::reject_merge_into_holder(source, dest)?;
        let skipped = merge::merge_into(source, dest, settings)?;
        return Ok(Placed::Merged {
            into: dest.to_path_buf(),
            skipped,
        });
    }
    replace::entry(source, dest, settings).map(|()| Placed::AsSent(dest.to_path_buf()))
}

/// "Keep both" in the "File already exists" question, which reaches here as
/// "Ask" (see [`crate::models::ClashAnswer::policy`]) — as does a name taken
/// after the question was answered: nothing already there is touched, and the
/// new one lands beside it under a free name.
fn place_beside(
    source: &Path,
    dest: &Path,
    settings: &crate::copy::CopyPrompt,
) -> io::Result<Placed> {
    let parent = dest.parent().unwrap_or(dest);
    let beside = unique_destination(parent, dest.file_name().unwrap_or_default())?;
    place(source, &beside, settings)?;
    Ok(Placed::Beside(beside))
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
/// thing left is to copy and then remove, which is what every mover does. A
/// rename refused for any other reason fails the move; see [`rename_within`].
pub(super) fn place(
    source: &Path,
    dest: &Path,
    settings: &crate::copy::CopyPrompt,
) -> io::Result<()> {
    if settings.purpose == Purpose::Move && rename_within(source, dest)? == Renamed::Moved {
        return Ok(());
    }
    if settings.purpose == Purpose::Move {
        crate::progress::copying_across(source);
    }
    // The copy itself gives every entry the times and modes the dialog
    // keeps (`CopyRules`), each folder once its contents are in.
    copy_entry(source, dest, settings.rules())?;
    if settings.purpose == Purpose::Move {
        remove_moved(source, dest)?;
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

/// Whether `source` and `dest` are the same directory entry in the same
/// folder, differing only in the case of the name.
pub(super) fn is_case_only_rename(source: &Path, dest: &Path) -> bool {
    let (Some(src_name), Some(dst_name)) = (source.file_name(), dest.file_name()) else {
        return false;
    };
    if src_name == dst_name
        || src_name.to_string_lossy().to_lowercase() != dst_name.to_string_lossy().to_lowercase()
    {
        return false;
    }
    let folder = replace::folder_of(dest);
    if replace::folder_of(source).canonicalize().ok() != folder.canonicalize().ok() {
        return false;
    }
    let same_entry = replace::are_hard_links(source, dest)
        || matches!(
            (source.canonicalize(), dest.canonicalize()),
            (Ok(real_src), Ok(real_dst)) if real_src == real_dst
        )
        || (cfg!(windows)
            && fs::symlink_metadata(source).is_ok()
            && fs::symlink_metadata(dest).is_ok());
    same_entry
        && fs::read_dir(folder).is_ok_and(|mut entries| {
            !entries.any(|entry| entry.is_ok_and(|e| e.file_name() == dst_name))
        })
}
