//! Purpose: how each `Status` reads — its `Display` impl, one formatter per outcome family, and the `String` conversion.

use std::borrow::Cow;
use std::ffi::OsStr;
use std::fmt;

use super::{FailedEntry, Landed, Status};
use crate::copy::Purpose;
use crate::path_display::label;
use std::path::Path;

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        [
            Self::format_path,
            Self::format_view,
            Self::format_transfer,
            Self::format_archive,
            Self::format_panel,
            Self::format_bookmark,
            Self::format_editor,
            Self::format_file,
            Self::format_vault,
        ]
        .into_iter()
        .find_map(|render| render(self, f))
        .unwrap_or(Err(fmt::Error))
    }
}

/// What an error's message starts with, whichever outcome it reports.
const ERROR_PREFIX: &str = "Error: ";

/// A partly failed batch: its `summary`, then the first failure — the path it
/// failed on, quoted so a narrow row shortens that and keeps the reason.
fn first_failure<'a>(summary: &str, first_error: &'a FailedEntry) -> PathMessage<'a> {
    PathMessage::quoting(
        format!("{summary}: "),
        label(first_error.path()),
        format!(": {}", first_error.error()),
    )
}

/// The one shape a partly failed copy, move, delete or wipe reports in: the
/// verb, the successes, the failures, and the first failure spelled out.
fn with_errors<'a>(
    verb: &str,
    count: usize,
    error_count: usize,
    first_error: &'a FailedEntry,
) -> PathMessage<'a> {
    first_failure(
        &format!("{verb} {count} item(s), {error_count} error(s)"),
        first_error,
    )
}

/// A message split around the path it quotes: the words before it, the path
/// as the message spells it, and the words after it. `path` is empty for a
/// message that quotes none.
///
/// Three parts rather than one line so a row too narrow for the whole message
/// can shorten the *path* — from its head, keeping the file name — and leave
/// the reason whole: `Cannot edit …/unreadable.txt: Permission denied`. A
/// line already flattened to one string can only be cut at its end, and the
/// end is where the file name and the reason are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathMessage<'a> {
    pub before: Cow<'a, str>,
    pub path: Cow<'a, str>,
    pub after: Cow<'a, str>,
}

impl<'a> PathMessage<'a> {
    /// A message quoting `path` between `before` and `after`; the words may
    /// be literals or formatted.
    pub fn quoting(
        before: impl Into<Cow<'a, str>>,
        path: impl Into<Cow<'a, str>>,
        after: impl Into<Cow<'a, str>>,
    ) -> Self {
        Self {
            before: before.into(),
            path: path.into(),
            after: after.into(),
        }
    }
}

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

/// "archive" or "archives", as `count` of them read.
const fn archive_noun(count: usize) -> &'static str {
    if count == 1 {
        "archive"
    } else {
        "archives"
    }
}

/// A message that quotes no path: all of it is `before`.
impl From<String> for PathMessage<'static> {
    fn from(text: String) -> Self {
        Self::quoting(text, "", "")
    }
}

impl fmt::Display for PathMessage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.before, self.path, self.after)
    }
}

impl Status {
    /// This outcome's message split around the path it quotes, or `None` for
    /// one that never quotes a path (every outcome the families below leave
    /// out). An outcome that quotes one only sometimes — a copy of several
    /// entries counts them — has an empty `path` then. The one place these
    /// messages are worded: `Display` prints exactly these parts.
    #[must_use]
    pub fn path_message(&self) -> Option<PathMessage<'_>> {
        self.view_path_message()
            .or_else(|| self.edit_path_message())
            .or_else(|| self.file_path_message())
            .or_else(|| self.archive_path_message())
            .or_else(|| self.run_path_message())
            .or_else(|| self.failure_path_message())
            .or_else(|| self.attributes_path_message())
    }

    fn view_path_message(&self) -> Option<PathMessage<'_>> {
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

    fn edit_path_message(&self) -> Option<PathMessage<'_>> {
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
    fn file_path_message(&self) -> Option<PathMessage<'_>> {
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

    /// Archive outcomes, named by the archive or the folder they unpacked to.
    fn archive_path_message(&self) -> Option<PathMessage<'_>> {
        Some(match self {
            Self::Extracted { count, destination } => PathMessage::quoting(
                format!("Extracted {count} {} to ", archive_noun(*count)),
                label(destination),
                "",
            ),
            Self::AddedToArchive {
                count,
                archive,
                kept,
            } => {
                let leftovers = if *kept > 0 {
                    format!("; {kept} could not be deleted")
                } else {
                    String::new()
                };
                PathMessage::quoting(
                    format!("Added {count} item(s) to "),
                    label(archive),
                    leftovers,
                )
            }
            _ => return None,
        })
    }

    /// How a program started with Enter ended, named by the program.
    fn run_path_message(&self) -> Option<PathMessage<'_>> {
        Some(match self {
            Self::ProgramExited { program, status } => {
                PathMessage::quoting("", label(program), format!(": {status}"))
            }
            Self::CannotRun { program, error } => PathMessage::quoting(
                format!("{ERROR_PREFIX}Cannot run "),
                label(program),
                format!(": {error}"),
            ),
            _ => return None,
        })
    }

    /// Copies, moves, deletes and wipes that partly failed, named by the
    /// first entry that failed.
    fn failure_path_message(&self) -> Option<PathMessage<'_>> {
        Some(match self {
            Self::CopiedWithErrors {
                count,
                error_count,
                first_error,
            } => with_errors("Copied", *count, *error_count, first_error),
            Self::MovedWithErrors {
                count,
                error_count,
                first_error,
            } => with_errors("Moved", *count, *error_count, first_error),
            Self::DeletedWithErrors {
                count,
                error_count,
                first_error,
            } => with_errors("Deleted", *count, *error_count, first_error),
            Self::WipedWithErrors {
                count,
                error_count,
                first_error,
            } => with_errors("Wiped", *count, *error_count, first_error),
            _ => return None,
        })
    }

    /// Ctrl+A that failed on some entries, named by the first.
    fn attributes_path_message(&self) -> Option<PathMessage<'_>> {
        let Self::AttributesSetWithErrors {
            count,
            error_count,
            first_error,
        } = self
        else {
            return None;
        };
        Some(first_failure(
            &format!("Attributes set on {count} object(s), {error_count} failed"),
            first_error,
        ))
    }

    fn format_path(&self, f: &mut fmt::Formatter<'_>) -> Option<fmt::Result> {
        self.path_message().map(|message| write!(f, "{message}"))
    }

    fn format_view(&self, f: &mut fmt::Formatter<'_>) -> Option<fmt::Result> {
        Some(match self {
            Self::Cleared => Ok(()),
            Self::KeyHints => write!(
                f,
                "F1 Help  F3 View  F4 Edit  F5 Copy  F6 Move  F7 Mkdir  F8 Delete"
            ),
            Self::CommandOutput(text) => f.write_str(text),
            Self::ConsoleStarted(command) => write!(f, "Started in new console: {command}"),
            Self::PackageUpdateCancelled => write!(f, "Package update cancelled"),
            Self::FileTooLarge {
                megabytes,
                limit_megabytes,
            } => write!(
                f,
                "File too large to view ({megabytes:.1} MB, max {limit_megabytes:.0} MB)"
            ),
            _ => return None,
        })
    }

    fn format_transfer(&self, f: &mut fmt::Formatter<'_>) -> Option<fmt::Result> {
        Some(match self {
            Self::Deleted { count } => write!(f, "Deleted {count} item(s)"),
            Self::Wiped { count } => write!(f, "Wiped {count} item(s)"),
            Self::WipedSharedFiles { count, shared } => write!(
                f,
                "Wiped {count} item(s); {shared} hard-linked file(s) only unlinked, \
                 their data stays under the other names"
            ),
            Self::AttributesSet { count } => write!(f, "Attributes set on {count} object(s)"),
            _ => return None,
        })
    }

    fn format_archive(&self, f: &mut fmt::Formatter<'_>) -> Option<fmt::Result> {
        Some(match self {
            Self::ExtractedWithErrors {
                done,
                total,
                first_error,
            } => write!(
                f,
                "Extracted {done} of {total} {}, first error: {first_error}",
                archive_noun(*total)
            ),
            _ => return None,
        })
    }

    fn format_panel(&self, f: &mut fmt::Formatter<'_>) -> Option<fmt::Result> {
        Some(match self {
            Self::OpenedView(title) => write!(f, "Opened {title}"),
            Self::PanelsSwapped => write!(f, "Panels swapped"),
            Self::PanelsRefreshed => write!(f, "Panels refreshed"),
            Self::RefreshFailed(error) => write!(f, "Refresh failed: {error}"),
            Self::Sort(label) => write!(f, "Sort: {label}"),
            Self::Found(query) => write!(f, "Found: {query}"),
            Self::NotFound(query) => write!(f, "Not found: {query}"),
            Self::Marked { count } => write!(f, "Marked {count} items"),
            Self::UnmarkedAll => write!(f, "Unmarked all"),
            _ => return None,
        })
    }

    fn format_bookmark(&self, f: &mut fmt::Formatter<'_>) -> Option<fmt::Result> {
        Some(match self {
            Self::OpenBookmarkFailed { slot, error } => {
                write!(f, "Failed to open bookmark {slot}: {error}")
            }
            Self::BookmarkEmpty { slot } => write!(f, "Bookmark {slot} is empty"),
            Self::InvalidBookmarkSlot { slot } => write!(f, "Invalid bookmark slot {slot}"),
            _ => return None,
        })
    }

    fn format_editor(&self, f: &mut fmt::Formatter<'_>) -> Option<fmt::Result> {
        Some(match self {
            Self::EditDirUnsupported => write!(f, "Editing directories is not supported"),
            Self::FileTooLargeEdit {
                megabytes,
                limit_megabytes,
            } => write!(
                f,
                "File too large to edit ({megabytes:.1} MB, max {limit_megabytes:.0} MB)"
            ),
            Self::EditorExited { status } => write!(f, "Editor exited with status {status}"),
            Self::LaunchEditorFailed { error } => write!(f, "Failed to launch editor: {error}"),
            Self::SaveFailed { error } => write!(f, "Save failed: {error}"),
            Self::ChangesDiscarded => write!(f, "Changes discarded"),
            _ => return None,
        })
    }

    fn format_file(&self, f: &mut fmt::Formatter<'_>) -> Option<fmt::Result> {
        Some(match self {
            Self::FilenameEmpty => write!(f, "Filename cannot be empty"),
            Self::CreatedFile(name) => write!(f, "Created file: {name}"),
            Self::CreateFileFailed { error } => write!(f, "Failed to create file: {error}"),
            Self::DeleteFailed { error } => write!(f, "Delete failed: {error}"),
            Self::DeleteCancelled => write!(f, "Delete cancelled"),
            Self::WipeCancelled => write!(f, "Wipe cancelled"),
            Self::NothingToWipe => write!(f, "Nothing to wipe"),
            Self::OpenDriveFailed { error } => write!(f, "Failed to open drive: {error}"),
            Self::S3ProfileSelected(profile) => {
                write!(
                    f,
                    "S3 profile '{profile}' selected (S3 browser coming soon)"
                )
            }
            _ => return None,
        })
    }

    fn format_vault(&self, f: &mut fmt::Formatter<'_>) -> Option<fmt::Result> {
        Some(match self {
            Self::VaultOpened(name) => write!(f, "Opened {name}: enter the master password"),
            Self::VaultLocked => write!(f, "Vault locked"),
            Self::VaultRemembered => write!(
                f,
                "Vault closed; key remembered for this session (5 min idle)"
            ),
            Self::VaultUnlocked { entries } => write!(f, "Vault unlocked: {entries} entries"),
            Self::VaultEntryDeleted => write!(f, "KeePass entry deleted"),
            Self::VaultChangeUndone => write!(f, "Last KeePass change undone"),
            Self::UnlockFailed => write!(f, "Unlock failed"),
            Self::Copied(secret) => write!(f, "{} copied", secret.label()),
            Self::CopiedViaOsc52(secret) => {
                write!(f, "{} sent to the terminal via OSC 52", secret.label())
            }
            Self::CopyFailed { error } => write!(f, "Copy failed: {error}"),
            Self::Error(error) => write!(f, "{ERROR_PREFIX}{error}"),
            Self::WindowsOnly { tool } => {
                write!(f, "Windows {tool} checker is only available on Windows")
            }
            _ => return None,
        })
    }
}

impl From<Status> for String {
    fn from(status: Status) -> Self {
        status.to_string()
    }
}
