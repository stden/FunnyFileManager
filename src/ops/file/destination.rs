//! Purpose: what the destination typed into the F5/F6 dialog means for each
//! item sent there — Far's rule for telling "put it in this folder" from
//! "give it this name". Kept apart from the transfer so the rule reads in one
//! place.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

/// Where the items of one F5/F6 transfer go.
///
/// A destination that is an existing folder, that ends in a separator (Far:
/// "if you want to create the destination folder before copying, append
/// backslash to its name"), or that receives several items is a folder to put
/// each of them in under its own name. Anything else is the new name of the
/// one item: that is how F6 renames, and how F5 makes a copy called something
/// else. A folder given a new name *becomes* that folder — its contents land
/// directly in it, as Far's help describes.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Destination {
    /// Put each item inside this folder, under the item's own name.
    Into(PathBuf),
    /// The one item becomes `name` inside `folder`.
    Renamed { folder: PathBuf, name: OsString },
}

impl Destination {
    /// Read `typed` for a transfer of `count` items. A relative path is taken
    /// against `base`, the active panel's folder.
    pub(super) fn parse(typed: &str, base: &Path, sources: &[PathBuf], count: usize) -> Self {
        let typed = typed.trim();
        let path = base.join(typed);
        let case_rename = count == 1
            && !typed.ends_with(std::path::is_separator)
            && sources
                .first()
                .is_some_and(|source| super::placement::is_case_only_rename(source, &path));
        let is_folder = !case_rename
            && (count > 1 || typed.ends_with(std::path::is_separator) || path.is_dir());
        match (is_folder, path.parent(), path.file_name()) {
            (false, Some(folder), Some(name)) => Self::Renamed {
                folder: folder.to_path_buf(),
                name: name.to_owned(),
            },
            _ => Self::Into(path),
        }
    }

    /// The folder `source` goes into, and the name it takes there.
    pub(super) fn for_source<'a>(&'a self, source: &'a Path) -> (&'a Path, &'a OsStr) {
        match self {
            Self::Into(folder) => (folder, source.file_name().unwrap_or_default()),
            Self::Renamed { folder, name } => (folder, name),
        }
    }
}
