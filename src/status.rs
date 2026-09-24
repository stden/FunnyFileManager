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

use crate::copy::Purpose;
use crate::models::{App, BookmarkSlot};
use std::path::PathBuf;

mod display;
mod failed_entry;
mod landed;
mod seam;
mod secret;

pub use display::PathMessage;
pub use failed_entry::FailedEntry;
pub use landed::Landed;
pub use secret::Secret;

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
    Path(PathBuf),
    /// Free-form text a command produced — a shell command's last stdout line,
    /// or its first stderr line. No outcome name can stand in for it, so the
    /// variant carries it and says so.
    CommandOutput(String),
    /// A command was started in its own console; it may still be running.
    ConsoleStarted(crate::command::ShellCommand),
    /// A program started with Enter ended, with this exit status.
    ProgramExited {
        program: PathBuf,
        status: std::process::ExitStatus,
    },
    /// A program started with Enter could not be started.
    CannotRun {
        program: PathBuf,
        error: String,
    },
    /// Ctrl+C or Ctrl+\ stopped Update packages while it had the terminal.
    PackageUpdateCancelled,

    // ── viewing / file operations ──────────────────────────────
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
    /// F7 made `count` folders in `directory`.
    CreatedDirs {
        count: usize,
        directory: PathBuf,
    },
    /// Rename, Make folder or Link found this name taken, and left it alone.
    AlreadyExists(PathBuf),
    /// A link was made at `link`, pointing at `target`.
    Linked {
        link: PathBuf,
        target: PathBuf,
    },
    /// Copies or moves that all went through: where to, and how many the
    /// F5 / F6 dialog's "already existing files" choice skipped or renamed.
    Transferred {
        purpose: Purpose,
        landed: Landed,
        skipped: usize,
        renamed: usize,
    },
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
    Deleted {
        count: usize,
    },
    /// Some deletions failed: success count, error count, first error message.
    DeletedWithErrors {
        count: usize,
        error_count: usize,
        first_error: FailedEntry,
    },
    /// `count` items wiped (zeroed, truncated, renamed, removed), no errors.
    Wiped {
        count: usize,
    },
    /// Wiped without errors, but `shared` files had other hard links: only
    /// their name went, and their data stays under the other names.
    WipedSharedFiles {
        count: usize,
        shared: usize,
    },
    /// Some wipes failed: success count, error count, first error message.
    WipedWithErrors {
        count: usize,
        error_count: usize,
        first_error: FailedEntry,
    },
    /// Ctrl+A set the dialog's attributes on `count` objects, no errors.
    AttributesSet {
        count: usize,
    },
    /// Ctrl+A set them on `count` objects and failed on `error_count`; the
    /// first failure.
    AttributesSetWithErrors {
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
    Marked {
        count: usize,
    },
    /// All marks were cleared.
    UnmarkedAll,

    // ── bookmarks ──────────────────────────────────────────────
    /// A bookmark slot was set to a path.
    BookmarkSet {
        slot: BookmarkSlot,
        path: PathBuf,
    },
    /// Jumped to a bookmarked path.
    OpenedBookmark {
        slot: BookmarkSlot,
        path: PathBuf,
    },
    /// Failed to load a bookmarked path.
    OpenBookmarkFailed {
        slot: BookmarkSlot,
        error: String,
    },
    /// A bookmark slot has no path.
    BookmarkEmpty {
        slot: BookmarkSlot,
    },
    /// A bookmark slot index was out of range / unparsable.
    InvalidBookmarkSlot {
        slot: String,
    },

    // ── editor ─────────────────────────────────────────────────
    /// Now editing the file at this path.
    Editing(String),
    /// Finished editing the file at this path.
    Edited(String),
    /// Could not edit a path (with the error).
    CannotEdit {
        path: PathBuf,
        error: String,
    },
    /// Tried to edit a directory.
    EditDirUnsupported,
    /// File exceeds the *edit* limit; carries its size and the limit, in MB.
    FileTooLargeEdit {
        megabytes: f64,
        limit_megabytes: f64,
    },
    /// The external editor exited with a non-zero status.
    EditorExited {
        status: String,
    },
    /// The external editor failed to launch.
    LaunchEditorFailed {
        error: String,
    },
    /// The built-in editor saved a file.
    Saved(PathBuf),
    /// The built-in editor failed to save.
    SaveFailed {
        error: String,
    },
    /// Edits were discarded.
    ChangesDiscarded,

    // ── archives ───────────────────────────────────────────────
    /// Extract files unpacked all `count` archives into `destination`.
    Extracted {
        count: usize,
        destination: PathBuf,
    },
    /// Extract files unpacked `done` of `total` archives; the first failure.
    ExtractedWithErrors {
        done: usize,
        total: usize,
        first_error: String,
    },
    /// Add to archive put `count` items into `archive`; `kept` of the sources
    /// it was to delete afterwards could not be.
    AddedToArchive {
        count: usize,
        archive: PathBuf,
        kept: usize,
    },

    // ── settings ───────────────────────────────────────────────
    /// Save setup wrote the settings file at this path.
    SetupSaved(PathBuf),

    // ── new file dialog ────────────────────────────────────────
    /// The new-file name was empty.
    FilenameEmpty,
    /// The target file already exists.
    FileExists(String),
    /// A file was created.
    CreatedFile(String),
    /// File creation failed.
    CreateFileFailed {
        error: String,
    },

    // ── delete confirmation ────────────────────────────────────
    /// A delete operation failed.
    DeleteFailed {
        error: String,
    },
    /// The delete was cancelled.
    DeleteCancelled,
    /// The wipe was cancelled.
    WipeCancelled,
    /// Alt+Del with the cursor on `..`: no dialog about nothing.
    NothingToWipe,

    // ── drives ─────────────────────────────────────────────────
    /// Opening a drive / S3 profile failed.
    OpenDriveFailed {
        error: String,
    },
    /// Changed the panel to a new path (drive selection).
    ChangedTo(PathBuf),
    /// An S3 profile was selected (browsing not yet implemented).
    S3ProfileSelected(String),

    // ── keepass vault ──────────────────────────────────────────
    /// The vault browser opened on this file, locked. The file name is the
    /// only payload; nothing decrypted exists yet to leak.
    VaultOpened(String),
    /// The vault re-locked and the master key was destroyed.
    VaultLocked,
    VaultRemembered,
    /// The master password unlocked the vault.
    VaultUnlocked {
        entries: usize,
    },
    VaultEntryDeleted,
    VaultChangeUndone,
    /// The master password was refused. Deliberately bare: the underlying
    /// error can describe the key material, so the detail goes beside the
    /// password field, never onto the status bar.
    UnlockFailed,
    /// A secret went to the system clipboard — named by its label, never by
    /// its value.
    Copied(Secret),
    /// A secret went to the terminal via OSC 52. A distinct outcome because
    /// terminals that disable OSC 52 drop the write silently, and the user
    /// must know which route to distrust.
    CopiedViaOsc52(Secret),
    /// A copy failed; `error` is pre-redacted by the producer and never
    /// carries the secret.
    CopyFailed {
        error: String,
    },

    // ── generic / platform ─────────────────────────────────────
    /// A generic command error.
    Error(String),
    /// A Windows-only tool was invoked on a non-Windows platform.
    WindowsOnly {
        tool: &'static str,
    },
}

/// Where the cursor stood when the message now on screen was drawn.
///
/// Far's info row carries a message only until the user moves on: the next
/// Up or Down puts the entry's name back. So a message has to remember which
/// cursor it belongs to — panel, directory and row, because any of the three
/// changing means the user is looking at something else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusCursor {
    side: crate::models::PanelSide,
    directory: PathBuf,
    row: Option<usize>,
}

/// The info row's lifetime for one outcome, independent of its message text.
#[derive(Debug, Clone, Default)]
pub(crate) enum StatusDisplay {
    #[default]
    Pending,
    Drawn(StatusCursor),
    Expired,
}

impl StatusCursor {
    /// Is this still where the cursor is? Compared in place rather than
    /// rebuilt, so an unchanged frame costs no allocation.
    fn is_current(&self, app: &App) -> bool {
        let panel = app.get_active_panel_ref();
        self.side == app.active_panel
            && self.directory == panel.path
            && self.row == panel.selected()
    }
}

#[cfg(test)]
mod tests;
