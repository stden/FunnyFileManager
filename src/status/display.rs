//! Purpose: how each `Status` reads — its `Display` impl, one formatter per outcome family, and the `String` conversion.

use std::fmt;

use super::{FailedEntry, Status};

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        [
            Self::format_view,
            Self::format_transfer,
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

/// The one shape every partly failed batch reports in: the verb, the
/// successes, the failures, and the first failure spelled out.
fn with_errors(
    f: &mut fmt::Formatter<'_>,
    verb: &str,
    count: usize,
    error_count: usize,
    first_error: &FailedEntry,
) -> fmt::Result {
    write!(
        f,
        "{verb} {count} item(s), {error_count} error(s): {first_error}"
    )
}

impl Status {
    fn format_view(&self, f: &mut fmt::Formatter<'_>) -> Option<fmt::Result> {
        Some(match self {
            Self::Cleared => Ok(()),
            Self::KeyHints => write!(
                f,
                "F1 Help  F3 View  F4 Edit  F5 Copy  F6 Move  F7 Mkdir  F8 Delete"
            ),
            Self::Path(path) => write!(f, "{}", path.display()),
            Self::CommandOutput(text) => f.write_str(text),
            Self::ConsoleStarted(command) => write!(f, "Started in new console: {command}"),
            Self::FileTooLarge {
                megabytes,
                limit_megabytes,
            } => write!(
                f,
                "File too large to view ({megabytes:.1} MB, max {limit_megabytes:.0} MB)"
            ),
            Self::BinaryFile(path) => write!(f, "Cannot view binary file {path}"),
            Self::NotRegularFile(path) => write!(f, "Cannot open {path}: not a regular file"),
            Self::Viewing(path) => write!(f, "Viewing {path}"),
            Self::CreatedDir(path) => write!(f, "Created directory {path}"),
            _ => return None,
        })
    }

    fn format_transfer(&self, f: &mut fmt::Formatter<'_>) -> Option<fmt::Result> {
        Some(match self {
            Self::CopiedWithErrors {
                count,
                error_count,
                first_error,
            } => with_errors(f, "Copied", *count, *error_count, first_error),
            Self::MovedWithErrors {
                count,
                error_count,
                first_error,
            } => with_errors(f, "Moved", *count, *error_count, first_error),
            Self::Deleted { count } => write!(f, "Deleted {count} item(s)"),
            Self::DeletedWithErrors {
                count,
                error_count,
                first_error,
            } => with_errors(f, "Deleted", *count, *error_count, first_error),
            Self::Wiped { count } => write!(f, "Wiped {count} item(s)"),
            Self::WipedWithErrors {
                count,
                error_count,
                first_error,
            } => with_errors(f, "Wiped", *count, *error_count, first_error),
            _ => return None,
        })
    }

    fn format_panel(&self, f: &mut fmt::Formatter<'_>) -> Option<fmt::Result> {
        Some(match self {
            Self::Opened(path) => write!(f, "Opened {}", path.display()),
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
            Self::BookmarkSet { slot, path } => {
                write!(f, "Bookmark {slot} set to {}", path.display())
            }
            Self::OpenedBookmark { slot, path } => {
                write!(f, "Opened bookmark {slot}: {}", path.display())
            }
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
            Self::Editing(path) => write!(f, "Editing {path}"),
            Self::Edited(path) => write!(f, "Edited {path}"),
            Self::CannotEdit { path, error } => {
                write!(f, "Cannot edit {}: {error}", path.display())
            }
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
            Self::Saved(path) => write!(f, "Saved {}", path.display()),
            Self::SaveFailed { error } => write!(f, "Save failed: {error}"),
            Self::ChangesDiscarded => write!(f, "Changes discarded"),
            _ => return None,
        })
    }

    fn format_file(&self, f: &mut fmt::Formatter<'_>) -> Option<fmt::Result> {
        Some(match self {
            Self::FilenameEmpty => write!(f, "Filename cannot be empty"),
            Self::FileExists(name) => write!(f, "File already exists: {name}"),
            Self::CreatedFile(name) => write!(f, "Created file: {name}"),
            Self::CreateFileFailed { error } => write!(f, "Failed to create file: {error}"),
            Self::DeleteFailed { error } => write!(f, "Delete failed: {error}"),
            Self::DeleteCancelled => write!(f, "Delete cancelled"),
            Self::WipeCancelled => write!(f, "Wipe cancelled"),
            Self::NothingToWipe => write!(f, "Nothing to wipe"),
            Self::OpenDriveFailed { error } => write!(f, "Failed to open drive: {error}"),
            Self::ChangedTo(path) => write!(f, "Changed to {}", path.display()),
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
            Self::Error(error) => write!(f, "Error: {error}"),
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
