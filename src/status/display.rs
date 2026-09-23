//! Purpose: how each `Status` reads — its `Display` impl, one formatter per outcome family, and the `String` conversion.

use std::borrow::Cow;
use std::fmt;

use super::{FailedEntry, Status};
use crate::path_display::label;
use std::path::Path;

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        [
            Self::format_path,
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

/// A [`PathMessage`] from its parts; the words may be literals or formatted.
fn quote<'a>(
    before: impl Into<Cow<'a, str>>,
    path: Cow<'a, str>,
    after: impl Into<Cow<'a, str>>,
) -> PathMessage<'a> {
    PathMessage {
        before: before.into(),
        path,
        after: after.into(),
    }
}

/// A message that quotes no path: all of it is `before`.
impl From<String> for PathMessage<'static> {
    fn from(text: String) -> Self {
        quote(text, Cow::Borrowed(""), "")
    }
}

impl fmt::Display for PathMessage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.before, self.path, self.after)
    }
}

impl Status {
    /// This outcome's message split around the path it quotes, or `None` for
    /// one that quotes no path (every outcome the two families below leave
    /// out). The one place these messages are worded: `Display` prints
    /// exactly these parts.
    #[must_use]
    pub fn path_message(&self) -> Option<PathMessage<'_>> {
        self.view_path_message()
            .or_else(|| self.edit_path_message())
    }

    fn view_path_message(&self) -> Option<PathMessage<'_>> {
        Some(match self {
            Self::Path(path) => quote("", label(path), ""),
            Self::BinaryFile(path) => quote("Cannot view binary file ", label(Path::new(path)), ""),
            Self::NotRegularFile(path) => quote(
                "Cannot open ",
                label(Path::new(path)),
                ": not a regular file",
            ),
            Self::Viewing(path) => quote("Viewing ", label(Path::new(path)), ""),
            Self::CreatedDir(path) => quote("Created directory ", label(Path::new(path)), ""),
            Self::Opened(path) => quote("Opened ", label(path), ""),
            Self::ChangedTo(path) => quote("Changed to ", label(path), ""),
            Self::BookmarkSet { slot, path } => {
                quote(format!("Bookmark {slot} set to "), label(path), "")
            }
            Self::OpenedBookmark { slot, path } => {
                quote(format!("Opened bookmark {slot}: "), label(path), "")
            }
            _ => return None,
        })
    }

    fn edit_path_message(&self) -> Option<PathMessage<'_>> {
        Some(match self {
            Self::Editing(path) => quote("Editing ", label(Path::new(path)), ""),
            Self::Edited(path) => quote("Edited ", label(Path::new(path)), ""),
            Self::CannotEdit { path, error } => {
                quote("Cannot edit ", label(path), format!(": {error}"))
            }
            Self::Saved(path) => quote("Saved ", label(path), ""),
            _ => return None,
        })
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
            Self::WipedSharedFiles { count, shared } => write!(
                f,
                "Wiped {count} item(s); {shared} hard-linked file(s) only unlinked, \
                 their data stays under the other names"
            ),
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
            Self::FileExists(name) => write!(f, "File already exists: {name}"),
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
