//! Purpose: path messages of outcomes named by the place they happened — a file viewed or edited, a folder opened or made, where a copy or move went.

use std::borrow::Cow;
use std::ffi::OsStr;
use std::path::Path;

use super::PathMessage;
use crate::copy::Purpose;
use crate::path_display::label;
use crate::status::{Landed, Status};

/// A copy or move that went through: the one place it names, or how many
/// went, then what the dialog's "already existing files" choice changed.
fn transfer_message(
    purpose: Purpose,
    landed: &Landed,
    skipped: usize,
    renamed: usize,
) -> PathMessage<'_> {
    let done = purpose.done();
    let skipped = if skipped > 0 {
        format!(", {skipped} skipped")
    } else {
        String::new()
    };
    let renamed = if renamed > 0 {
        " (existing files kept, new ones renamed)"
    } else {
        ""
    };
    let after = format!("{skipped}{renamed}");
    match landed {
        Landed::At(path) => PathMessage::quoting(format!("{done} to "), label(path), after),
        Landed::Many(count) => {
            PathMessage::quoting(format!("{done} {count} item(s)"), Cow::Borrowed(""), after)
        }
        Landed::Nowhere => {
            PathMessage::quoting(format!("{done} nothing"), Cow::Borrowed(""), after)
        }
    }
}

impl Status {
    pub(super) fn view_path_message(&self) -> Option<PathMessage<'_>> {
        Some(match self {
            Self::Path(path) => PathMessage::quoting("", label(path), ""),
            Self::BinaryFile(path) => {
                PathMessage::quoting("Cannot view binary file ", label(Path::new(path)), "")
            }
            Self::NotRegularFile(path) => PathMessage::quoting(
                "Cannot open ",
                label(Path::new(path)),
                ": not a regular file",
            ),
            Self::Viewing(path) => PathMessage::quoting("Viewing ", label(Path::new(path)), ""),
            Self::CreatedDir(path) => {
                PathMessage::quoting("Created directory ", label(Path::new(path)), "")
            }
            Self::Opened(path) => PathMessage::quoting("Opened ", label(path), ""),
            Self::ChangedTo(path) => PathMessage::quoting("Changed to ", label(path), ""),
            Self::BookmarkSet { slot, path } => {
                PathMessage::quoting(format!("Bookmark {slot} set to "), label(path), "")
            }
            Self::OpenedBookmark { slot, path } => {
                PathMessage::quoting(format!("Opened bookmark {slot}: "), label(path), "")
            }
            _ => return None,
        })
    }

    pub(super) fn edit_path_message(&self) -> Option<PathMessage<'_>> {
        Some(match self {
            Self::Editing(path) => PathMessage::quoting("Editing ", label(Path::new(path)), ""),
            Self::Edited(path) => PathMessage::quoting("Edited ", label(Path::new(path)), ""),
            Self::CannotEdit { path, error } => {
                PathMessage::quoting("Cannot edit ", label(path), format!(": {error}"))
            }
            Self::Saved(path) => PathMessage::quoting("Saved ", label(path), ""),
            _ => return None,
        })
    }

    /// Outcomes that made a file or folder, or found its name taken, named by
    /// where.
    pub(super) fn file_path_message(&self) -> Option<PathMessage<'_>> {
        Some(match self {
            Self::SetupSaved(path) => PathMessage::quoting("Setup saved to ", label(path), ""),
            Self::Transferred {
                purpose,
                landed,
                skipped,
                renamed,
            } => transfer_message(*purpose, landed, *skipped, *renamed),
            Self::CreatedDirs { count, directory } => {
                PathMessage::quoting(format!("Created {count} folders in "), label(directory), "")
            }
            Self::AlreadyExists(path) => PathMessage::quoting("", label(path), " already exists"),
            Self::FileExists(name) => {
                PathMessage::quoting("File already exists: ", label(Path::new(name)), "")
            }
            // The link is named by its name alone: it sits in the other
            // panel, under the cursor. Its target is the path that can be
            // long — Alt+F6 fills in an absolute one.
            Self::Linked { link, target } => {
                let name = link
                    .file_name()
                    .map_or_else(|| label(link), OsStr::to_string_lossy);
                PathMessage::quoting(format!("Link {name} -> "), label(target), "")
            }
            _ => return None,
        })
    }
}
