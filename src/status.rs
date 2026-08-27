//! Typed application status outcomes — a small DSL for the status bar.
//!
//! Handlers, the editor, and file operations *produce* a [`Status`] describing
//! what happened; this module is the single place that *formats* it into the
//! user-facing line shown in the status bar (its [`Display`](std::fmt::Display)
//! impl). Separating "what happened" from "how it reads" keeps all message text
//! in one place (DRY) and lets tests match a typed outcome instead of a brittle
//! substring.
//!
//! Set it through [`App::set_status`], the seam between the operation layer and
//! the presentation layer. Genuinely free-form output (a shell command's stdout,
//! `pwd`'s bare path) is not an *outcome* and cannot be named by one — but it
//! still travels as a [`Status`], through variants that carry the text
//! verbatim. That is what makes `App::status` a single channel: before, two
//! call sites assigned the field directly and bypassed this module entirely.

use std::fmt;

use crate::models::{App, BookmarkSlot};
use std::path::PathBuf;

/// One file an operation could not process, and why.
///
/// A struct rather than a pre-formatted `"path: error"` string: the two halves
/// were joined at the point of failure, which threw away the ability to show
/// them differently — a name in the status bar, the full path in a dialog, the
/// error alone in a log. The rendering lives in `Display`, so there is one of
/// it, and nothing can put a bare error message where a failed entry belongs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FailedEntry {
    path: std::path::PathBuf,
    error: String,
}

impl FailedEntry {
    #[must_use]
    pub fn new(path: impl Into<std::path::PathBuf>, error: &dyn std::fmt::Display) -> Self {
        Self {
            path: path.into(),
            error: error.to_string(),
        }
    }

    #[must_use]
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    #[must_use]
    pub fn error(&self) -> &str {
        &self.error
    }
}

impl fmt::Display for FailedEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.error)
    }
}

/// A user-facing application outcome shown in the status bar.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Status {
    // ── not outcomes, but still the status bar's only input ────
    /// Nothing to report.
    #[default]
    Cleared,
    /// The F-key reminder a fresh session opens with.
    KeyHints,
    /// `pwd`: the active panel's path, verbatim.
    Path(std::path::PathBuf),
    /// Free-form text a command produced — a shell command's last stdout line,
    /// or its first stderr line. No outcome name can stand in for it, so the
    /// variant carries it and says so.
    CommandOutput(String),

    // ── viewing / file operations ──────────────────────────────
    /// F3 on a directory.
    CannotViewDir,
    /// File exceeds the *view* limit; carries its size and the limit, in MB.
    FileTooLarge {
        megabytes: f64,
        limit_megabytes: f64,
    },
    /// F3 on a file the line-oriented viewer cannot show.
    BinaryFile(String),
    /// F3 / F4 on something that is not a regular file — a FIFO, socket, or
    /// device node. Shared by both, since the reason is the same for both.
    NotRegularFile(String),
    /// Now viewing the file at this path.
    Viewing(String),
    /// A directory was created at this path.
    CreatedDir(String),
    /// Some copies failed: success count, error count, first error message.
    CopiedWithErrors {
        count: usize,
        error_count: usize,
        first_error: FailedEntry,
    },
    /// Some moves failed: success count, error count, first error message.
    MovedWithErrors {
        count: usize,
        error_count: usize,
        first_error: FailedEntry,
    },
    /// `count` items deleted, no errors.
    Deleted { count: usize },
    /// Some deletions failed: success count, error count, first error message.
    DeletedWithErrors {
        count: usize,
        error_count: usize,
        first_error: FailedEntry,
    },

    // ── navigation / panels ────────────────────────────────────
    /// Opened the directory at this path.
    ///
    /// A `PathBuf`, not a rendered string: the path arrived here already
    /// flattened by `.display().to_string()`, so nothing downstream could
    /// shorten it, show only its last component, or compare it.
    Opened(PathBuf),
    /// Opened a named report view (the Windows diagnostics), which has a title
    /// rather than a path. It used to share `Opened`, which is how that
    /// variant ended up documented as "this path *or* title".
    OpenedView(String),
    /// Panels' contents were swapped.
    PanelsSwapped,
    /// Both panels were reloaded.
    PanelsRefreshed,
    /// A panel refresh failed.
    RefreshFailed(String),
    /// Sort mode changed; carries the mode label.
    Sort(String),
    /// Quick-search found a match.
    Found(String),
    /// Quick-search found nothing.
    NotFound(String),
    /// `count` items are marked.
    Marked { count: usize },
    /// All marks were cleared.
    UnmarkedAll,

    // ── bookmarks ──────────────────────────────────────────────
    /// A bookmark slot was set to a path.
    BookmarkSet { slot: BookmarkSlot, path: PathBuf },
    /// Jumped to a bookmarked path.
    OpenedBookmark { slot: BookmarkSlot, path: PathBuf },
    /// Failed to load a bookmarked path.
    OpenBookmarkFailed { slot: BookmarkSlot, error: String },
    /// A bookmark slot has no path.
    BookmarkEmpty { slot: BookmarkSlot },
    /// A bookmark slot index was out of range / unparseable.
    InvalidBookmarkSlot { slot: String },

    // ── editor ─────────────────────────────────────────────────
    /// Now editing the file at this path.
    Editing(String),
    /// Finished editing the file at this path.
    Edited(String),
    /// Could not edit a path (with the error).
    CannotEdit { path: PathBuf, error: String },
    /// Tried to edit a directory.
    EditDirUnsupported,
    /// File exceeds the *edit* limit; carries its size and the limit, in MB.
    FileTooLargeEdit {
        megabytes: f64,
        limit_megabytes: f64,
    },
    /// The external editor exited with a non-zero status.
    EditorExited { status: String },
    /// The external editor failed to launch.
    LaunchEditorFailed { error: String },
    /// The built-in editor saved a file.
    Saved(PathBuf),
    /// The built-in editor failed to save.
    SaveFailed { error: String },
    /// Edits were discarded.
    ChangesDiscarded,

    // ── new file dialog ────────────────────────────────────────
    /// The new-file name was empty.
    FilenameEmpty,
    /// The target file already exists.
    FileExists(String),
    /// A file was created.
    CreatedFile(String),
    /// File creation failed.
    CreateFileFailed { error: String },

    // ── delete confirmation ────────────────────────────────────
    /// A delete operation failed.
    DeleteFailed { error: String },
    /// The delete was cancelled.
    DeleteCancelled,

    // ── drives ─────────────────────────────────────────────────
    /// Opening a drive / S3 profile failed.
    OpenDriveFailed { error: String },
    /// Changed the panel to a new path (drive selection).
    ChangedTo(PathBuf),
    /// Changed the panel to a drive by letter.
    ChangedToDrive(PathBuf),
    /// An S3 profile was selected (browsing not yet implemented).
    S3ProfileSelected(String),

    // ── generic / platform ─────────────────────────────────────
    /// A generic command error.
    Error(String),
    /// A Windows-only tool was invoked on a non-Windows platform.
    WindowsOnly { tool: &'static str },
}

impl fmt::Display for Status {
    #[allow(clippy::too_many_lines)] // one arm per outcome — kept flat on purpose
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cleared => Ok(()),
            Self::KeyHints => write!(
                f,
                "F1 Help  F3 View  F4 Edit  F5 Copy  F6 Move  F7 Mkdir  F8 Delete"
            ),
            Self::Path(path) => write!(f, "{}", path.display()),
            Self::CommandOutput(text) => f.write_str(text),
            Self::CannotViewDir => write!(f, "Cannot view a directory"),
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
            Self::CopiedWithErrors {
                count,
                error_count,
                first_error,
            } => write!(
                f,
                "Copied {count} item(s), {error_count} error(s): {first_error}"
            ),
            Self::MovedWithErrors {
                count,
                error_count,
                first_error,
            } => write!(
                f,
                "Moved {count} item(s), {error_count} error(s): {first_error}"
            ),
            Self::Deleted { count } => write!(f, "Deleted {count} item(s)"),
            Self::DeletedWithErrors {
                count,
                error_count,
                first_error,
            } => write!(
                f,
                "Deleted {count} item(s), {error_count} error(s): {first_error}"
            ),
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
            Self::FilenameEmpty => write!(f, "Filename cannot be empty"),
            Self::FileExists(name) => write!(f, "File already exists: {name}"),
            Self::CreatedFile(name) => write!(f, "Created file: {name}"),
            Self::CreateFileFailed { error } => write!(f, "Failed to create file: {error}"),
            Self::DeleteFailed { error } => write!(f, "Delete failed: {error}"),
            Self::DeleteCancelled => write!(f, "Delete cancelled"),
            Self::OpenDriveFailed { error } => write!(f, "Failed to open drive: {error}"),
            Self::ChangedTo(path) => write!(f, "Changed to {}", path.display()),
            Self::ChangedToDrive(path) => write!(f, "Changed to drive {}", path.display()),
            Self::S3ProfileSelected(profile) => {
                write!(
                    f,
                    "S3 profile '{profile}' selected (S3 browser coming soon)"
                )
            }
            Self::Error(error) => write!(f, "Error: {error}"),
            Self::WindowsOnly { tool } => {
                write!(f, "Windows {tool} checker is only available on Windows")
            }
        }
    }
}

impl From<Status> for String {
    fn from(status: Status) -> Self {
        status.to_string()
    }
}

impl App {
    /// Set the status-bar message from a typed [`Status`] outcome.
    ///
    /// This is the one place handlers should go through to report an outcome —
    /// they name *what happened*; [`Status`] decides *how it reads*.
    pub fn set_status(&mut self, status: Status) {
        self.status = status;
    }

    /// The status bar's current line.
    ///
    /// Rendered on demand rather than stored: keeping the `Status` means a test
    /// can match the outcome instead of a substring, and the formatting stays
    /// in one place.
    #[must_use]
    pub fn status_text(&self) -> String {
        self.status.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_entry_keeps_the_path_and_the_reason_apart() {
        // Joined into one string at the point of failure, they could only ever
        // be shown one way; kept apart, a dialog can show the full path while
        // the status bar shows the summary.
        let failure = FailedEntry::new("/srv/work/notes.txt", &"Permission denied");
        assert_eq!(failure.path(), std::path::Path::new("/srv/work/notes.txt"));
        assert_eq!(failure.error(), "Permission denied");
        assert_eq!(
            failure.to_string(),
            "/srv/work/notes.txt: Permission denied"
        );
    }

    #[test]
    fn a_failed_entry_takes_its_reason_from_any_error() {
        // `&dyn Display` so an `io::Error` needs no formatting at the call
        // site — that formatting is what used to lose the path.
        let error = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let failure = FailedEntry::new("/tmp/x", &error);
        assert_eq!(failure.error(), "gone");
    }

    /// The status bar has exactly one input.
    ///
    /// Two call sites used to assign `App.status_message` directly, bypassing
    /// this module — and one bdd step silently wrote to a temporary once the
    /// field became a `Status`. Keeping the outcome rather than its rendering
    /// is what lets a test match the outcome.
    #[test]
    fn free_form_output_still_travels_as_a_status() {
        assert_eq!(Status::Cleared.to_string(), "");
        assert_eq!(
            Status::Path(std::path::PathBuf::from("/srv/work")).to_string(),
            "/srv/work"
        );
        assert_eq!(
            Status::CommandOutput("total 0".to_owned()).to_string(),
            "total 0"
        );
        assert!(Status::KeyHints.to_string().contains("F1 Help"));
    }

    #[test]
    fn a_fresh_app_opens_on_the_key_hints() {
        let app = App::new().expect("app");
        assert_eq!(app.status, Status::KeyHints);
        assert!(app.status_text().contains("F5 Copy"));
    }

    #[test]
    fn set_status_keeps_the_outcome_not_its_rendering() {
        let mut app = App::new().expect("app");
        app.set_status(Status::UnmarkedAll);
        assert_eq!(app.status, Status::UnmarkedAll);
    }

    /// Locks every variant's rendered text to its historical `format!` string,
    /// so the migration away from stringly-typed messages cannot drift.
    #[test]
    fn display_matches_legacy_strings() {
        let cases: &[(Status, &str)] = &[
            (Status::CannotViewDir, "Cannot view a directory"),
            (
                Status::FileTooLarge {
                    megabytes: 12.34,
                    limit_megabytes: 10.0,
                },
                "File too large to view (12.3 MB, max 10 MB)",
            ),
            (
                Status::BinaryFile("/bin/ls".into()),
                "Cannot view binary file /bin/ls",
            ),
            (
                Status::NotRegularFile("/dev/zero".into()),
                "Cannot open /dev/zero: not a regular file",
            ),
            (Status::Viewing("/x".into()), "Viewing /x"),
            (Status::CreatedDir("/d".into()), "Created directory /d"),
            (
                Status::CopiedWithErrors {
                    count: 2,
                    error_count: 1,
                    first_error: FailedEntry::new("/srv/x", &"boom"),
                },
                "Copied 2 item(s), 1 error(s): /srv/x: boom",
            ),
            (
                Status::MovedWithErrors {
                    count: 2,
                    error_count: 1,
                    first_error: FailedEntry::new("/srv/x", &"boom"),
                },
                "Moved 2 item(s), 1 error(s): /srv/x: boom",
            ),
            (Status::Deleted { count: 3 }, "Deleted 3 item(s)"),
            (
                Status::DeletedWithErrors {
                    count: 2,
                    error_count: 1,
                    first_error: FailedEntry::new("/srv/x", &"boom"),
                },
                "Deleted 2 item(s), 1 error(s): /srv/x: boom",
            ),
            (Status::Opened(PathBuf::from("/p")), "Opened /p"),
            (Status::PanelsSwapped, "Panels swapped"),
            (Status::PanelsRefreshed, "Panels refreshed"),
            (Status::RefreshFailed("io".into()), "Refresh failed: io"),
            (Status::Sort("Name".into()), "Sort: Name"),
            (Status::Found("q".into()), "Found: q"),
            (Status::NotFound("q".into()), "Not found: q"),
            (Status::Marked { count: 4 }, "Marked 4 items"),
            (Status::UnmarkedAll, "Unmarked all"),
            (
                Status::BookmarkSet {
                    slot: BookmarkSlot::new(2).expect("legal slot"),
                    path: PathBuf::from("/b"),
                },
                "Bookmark 2 set to /b",
            ),
            (
                Status::OpenedBookmark {
                    slot: BookmarkSlot::new(2).expect("legal slot"),
                    path: PathBuf::from("/b"),
                },
                "Opened bookmark 2: /b",
            ),
            (
                Status::OpenBookmarkFailed {
                    slot: BookmarkSlot::new(2).expect("legal slot"),
                    error: "io".into(),
                },
                "Failed to open bookmark 2: io",
            ),
            (
                Status::BookmarkEmpty {
                    slot: BookmarkSlot::new(5).expect("legal slot"),
                },
                "Bookmark 5 is empty",
            ),
            (
                Status::InvalidBookmarkSlot { slot: "9".into() },
                "Invalid bookmark slot 9",
            ),
            (Status::Editing("/e".into()), "Editing /e"),
            (Status::Edited("/e".into()), "Edited /e"),
            (
                Status::CannotEdit {
                    path: PathBuf::from("/e"),
                    error: "nope".into(),
                },
                "Cannot edit /e: nope",
            ),
            (
                Status::EditDirUnsupported,
                "Editing directories is not supported",
            ),
            (
                Status::FileTooLargeEdit {
                    megabytes: 12.34,
                    limit_megabytes: 10.0,
                },
                "File too large to edit (12.3 MB, max 10 MB)",
            ),
            (
                Status::EditorExited {
                    status: "exit status: 1".into(),
                },
                "Editor exited with status exit status: 1",
            ),
            (
                Status::LaunchEditorFailed { error: "x".into() },
                "Failed to launch editor: x",
            ),
            (Status::Saved(PathBuf::from("/s")), "Saved /s"),
            (Status::SaveFailed { error: "x".into() }, "Save failed: x"),
            (Status::ChangesDiscarded, "Changes discarded"),
            (Status::FilenameEmpty, "Filename cannot be empty"),
            (
                Status::FileExists("a.txt".into()),
                "File already exists: a.txt",
            ),
            (Status::CreatedFile("a.txt".into()), "Created file: a.txt"),
            (
                Status::CreateFileFailed { error: "x".into() },
                "Failed to create file: x",
            ),
            (
                Status::DeleteFailed { error: "x".into() },
                "Delete failed: x",
            ),
            (Status::DeleteCancelled, "Delete cancelled"),
            (
                Status::OpenDriveFailed { error: "x".into() },
                "Failed to open drive: x",
            ),
            (Status::ChangedTo(PathBuf::from("/p")), "Changed to /p"),
            (
                Status::ChangedToDrive(PathBuf::from("C:\\")),
                "Changed to drive C:\\",
            ),
            (
                Status::S3ProfileSelected("dev".into()),
                "S3 profile 'dev' selected (S3 browser coming soon)",
            ),
            (Status::Error("x".into()), "Error: x"),
            (
                Status::WindowsOnly { tool: "system" },
                "Windows system checker is only available on Windows",
            ),
        ];
        for (status, expected) in cases {
            assert_eq!(&status.to_string(), expected, "mismatch for {status:?}");
        }
    }

    /// The limit in the message must follow the limit that was actually
    /// enforced — it used to be hard-coded as "max 10 MB" in the text.
    #[test]
    fn too_large_messages_report_the_carried_limit() {
        assert_eq!(
            Status::FileTooLarge {
                megabytes: 60.0,
                limit_megabytes: 50.0,
            }
            .to_string(),
            "File too large to view (60.0 MB, max 50 MB)"
        );
        assert_eq!(
            Status::FileTooLargeEdit {
                megabytes: 60.0,
                limit_megabytes: 50.0,
            }
            .to_string(),
            "File too large to edit (60.0 MB, max 50 MB)"
        );
    }

    /// The limit the handlers pass comes from one constant, so the rendered
    /// message and the guard cannot drift apart.
    #[test]
    fn view_limit_message_matches_the_constant() {
        let limit = crate::utils::MAX_FILE_SIZE.megabytes();
        assert_eq!(
            Status::FileTooLarge {
                megabytes: limit + 1.0,
                limit_megabytes: limit,
            }
            .to_string(),
            "File too large to view (11.0 MB, max 10 MB)"
        );
    }
}
