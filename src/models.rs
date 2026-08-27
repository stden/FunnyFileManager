//! Purpose: Defines core app state, domain enums, and panel data models.
//!
use std::{
    io,
    path::{Path, PathBuf},
};

use crate::console::ConsoleLog;
use crate::history::RingIndex;
use crate::lineedit::LineEditor;
use crate::quicksearch::QuickSearch;
use crate::scroll::ScrollTop;

#[cfg(windows)]
unsafe extern "system" {
    fn GetLogicalDrives() -> u32;
}

/// Kept for backward compatibility. Prefer `BookmarkSlot::COUNT`.
pub const MAX_BOOKMARKS: usize = BookmarkSlot::COUNT;

mod editor;
mod panel;

pub use editor::{CharColumn, EditState, Editor, FileFormat};
pub use panel::Panel;

/// A single entry in the file list (file or directory)
///
/// `is_dir` describes what the entry *behaves* like — a symlink to a directory
/// is entered, copied and deleted as a directory would be — while `is_symlink`
/// records that the entry is a link, which the raw `is_dir` flag would
/// otherwise lose. Both are needed: a broken link is neither a directory nor a
/// plain file.
#[derive(Debug, Clone, Default)]
pub struct DirEntry {
    pub name: String,
    pub kind: EntryKind,
    pub size: FileSize,
}

/// What a panel row *is*, as far as every operation on it is concerned.
///
/// This replaced a pair of `bool`s whose reading needed two paragraphs of
/// prose: `is_dir` described the symlink's *target* while `is_symlink`
/// recorded the link, and a broken link had to keep `is_dir == false`. Four
/// combinations, three explanations, and one shipped bug — a symlinked
/// directory rendered as a tiny file that could not be opened but whose whole
/// target tree got copied by F5. The states are named now.
///
/// The old encoding also could not tell a symlink-to-a-file from a dangling
/// one; both were `(false, true)`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EntryKind {
    #[default]
    File,
    Directory,
    /// A symlink whose target is a file.
    SymlinkToFile,
    /// A symlink whose target is a directory: entered, copied and deleted as a
    /// directory would be.
    SymlinkToDirectory,
    /// A symlink with nothing at the other end. Not a directory — there is
    /// nothing to enter — and not a file either.
    BrokenSymlink,
}

impl EntryKind {
    /// Whether this behaves as a directory: `Enter` descends into it, and file
    /// operations recurse through it.
    #[must_use]
    pub const fn is_dir(self) -> bool {
        matches!(self, Self::Directory | Self::SymlinkToDirectory)
    }

    /// Whether a link is involved, whatever it points at.
    #[must_use]
    pub const fn is_symlink(self) -> bool {
        matches!(
            self,
            Self::SymlinkToFile | Self::SymlinkToDirectory | Self::BrokenSymlink
        )
    }
}

/// A file's size in bytes.
///
/// Named because a bare `u64` in this program is just as likely to be a line
/// count, a byte offset or a cell budget — and because the megabyte conversion
/// and the human-readable rendering belong with the number rather than in free
/// functions anyone can call with any integer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileSize(u64);

impl FileSize {
    #[must_use]
    pub const fn from_bytes(bytes: u64) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn bytes(self) -> u64 {
        self.0
    }

    /// The size in megabytes — the unit the status bar reports in.
    #[must_use]
    pub fn megabytes(self) -> f64 {
        self.0 as f64 / (1024.0 * 1024.0)
    }
}

impl std::fmt::Display for FileSize {
    /// Human-readable, as the panel's size column shows it: "1.50 KB",
    /// "3.20 MB", or a bare byte count below a kibibyte.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        let size = self.0;
        if size >= GB {
            write!(f, "{:.2} GB", size as f64 / GB as f64)
        } else if size >= MB {
            write!(f, "{:.2} MB", size as f64 / MB as f64)
        } else if size >= KB {
            write!(f, "{:.2} KB", size as f64 / KB as f64)
        } else {
            write!(f, "{size} B")
        }
    }
}

/// How files are sorted in the panel: by name, extension, or size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum SortMode {
    #[default]
    Name,
    Extension,
    Size,
}

impl SortMode {
    #[allow(dead_code)]
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Name => Self::Extension,
            Self::Extension => Self::Size,
            Self::Size => Self::Name,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Extension => "Ext",
            Self::Size => "Size",
        }
    }
}

/// Built-in file viewer, opened with F3
#[derive(Default, Clone)]
pub struct Viewer {
    pub title: String,
    pub lines: Vec<String>,
    /// Anchored at the *top*, unlike the console transcript's offset — see
    /// [`crate::scroll`].
    pub scroll: ScrollTop,
}

/// What a completion candidate is, which decides how accepting it ends.
///
/// A `Word` gets a trailing space; a `Path` gets one too unless it is a
/// directory (ending in `/`), where the user is about to descend into it.
/// Lives here rather than in `complete` so `models` stays dependency-free.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateKind {
    Word,
    Path,
}

/// One row of the completion dropdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// Replacement text for the token being completed.
    pub text: String,
    /// Decides the trailing space when this row is accepted.
    pub kind: CandidateKind,
    /// Extra detail shown right-aligned — e.g. the path behind a bookmark slot.
    pub hint: Option<String>,
}

/// Open completion dropdown: the candidates and which one is highlighted.
///
/// Both fields are private because they share an invariant: the dropdown is
/// **never empty**, and `selected` always points at one of its rows. As public
/// fields, replacing `candidates` without resetting `selected` left the
/// highlight pointing past the end — safe only because the one code path that
/// refreshes a menu happens to rebuild the whole struct. [`Menu::new`] rejects
/// an empty candidate list instead, which is what makes [`Menu::selection`]
/// able to return a `&Candidate` rather than an `Option`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Menu {
    candidates: Vec<Candidate>,
    selected: usize,
}

impl Menu {
    /// A dropdown over `candidates`, highlighting the best-ranked row.
    /// `None` for an empty list: there is no dropdown to show.
    #[must_use]
    pub fn new(candidates: Vec<Candidate>) -> Option<Self> {
        (!candidates.is_empty()).then_some(Self {
            candidates,
            selected: 0,
        })
    }

    #[must_use]
    pub fn candidates(&self) -> &[Candidate] {
        &self.candidates
    }

    #[must_use]
    pub const fn selected(&self) -> usize {
        self.selected
    }

    /// The highlighted candidate. Total: the list cannot be empty.
    #[must_use]
    pub fn selection(&self) -> &Candidate {
        &self.candidates[self.selected]
    }

    /// Highlight the next candidate, wrapping at the end.
    pub const fn next(&mut self) {
        self.selected = (self.selected + 1) % self.candidates.len();
    }

    /// Highlight the previous candidate, wrapping at the start.
    pub fn previous(&mut self) {
        self.selected = self
            .selected
            .checked_sub(1)
            .unwrap_or(self.candidates.len() - 1);
    }
}

/// How the command line is currently consulting the history ring.
///
/// An enum rather than an `Option<usize>` beside an `Option<String>` beside an
/// `Option<query>`, for the reason [`Overlay`] is one: browsing with `Up`/`Down`
/// and reverse-searching with `Ctrl+R` are mutually exclusive, and `stash` — the
/// half-typed line that the first `Up` displaced — is meaningless outside
/// `Browsing`. Three `Option`s would make "browsing while searching" and
/// "browsing with no stash" representable states nothing knows how to render.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Recall {
    /// Not consulting the history; the buffer is whatever was typed.
    #[default]
    Idle,
    /// Walking back through the ring. `index` is the offset from the newest
    /// entry; `stash` is the line that was being typed when `Up` was first
    /// pressed, restored on walking forward past the newest entry.
    Browsing { index: RingIndex, stash: String },
    /// `Ctrl+R` reverse search. `hit` is the ring index currently shown, or
    /// `None` when nothing matches the query yet; `stash` is the line the
    /// search displaced, put back when the search is abandoned.
    Searching {
        query: String,
        hit: Option<RingIndex>,
        stash: String,
    },
}

/// State of the `:` command line: the editable line, any open completion
/// dropdown, the ghost suggestion, and the history-recall mode.
///
/// A struct rather than a bare `String` because the completion dropdown's
/// state belongs beside the buffer it completes: the two are edited by the
/// same keystrokes and must never drift apart. The line itself is a
/// [`LineEditor`] rather than a `String` so the caret travels with the text —
/// a separate cursor field could be left pointing past a shortened buffer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandLine {
    pub line: LineEditor,
    /// `None` while the dropdown is closed.
    pub menu: Option<Menu>,
    /// History suggestion tail beyond the typed text (grey text after the
    /// cursor). `None` when nothing matches, while the menu is open, or during
    /// a reverse search — the menu and the ghost must never compete for the
    /// same accept gesture.
    pub ghost: Option<String>,
    /// Whether `Up`/`Down` or `Ctrl+R` is currently driving the buffer.
    pub recall: Recall,
}

impl CommandLine {
    /// The text typed so far.
    #[must_use]
    pub fn text(&self) -> &str {
        self.line.text()
    }
}

impl From<String> for CommandLine {
    fn from(buffer: String) -> Self {
        Self {
            line: LineEditor::from(buffer),
            menu: None,
            ghost: None,
            recall: Recall::Idle,
        }
    }
}

impl From<&str> for CommandLine {
    fn from(buffer: &str) -> Self {
        Self {
            line: LineEditor::from(buffer),
            menu: None,
            ghost: None,
            recall: Recall::Idle,
        }
    }
}

/// A bare directory-entry name — never a path.
///
/// Marks are name-keyed, and a mark only means anything relative to the
/// directory that was listed when it was made; that is why `navigate_to` drops
/// them. Storing them as plain `String`s left that distinction to a comment,
/// and a comment cannot stop a `PathBuf::display().to_string()` being inserted
/// into the set. `Borrow<str>` keeps lookups by `&str` allocation-free, and the
/// derived `Ord` matches `str`'s because the type is one `String`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FileName(String);

impl FileName {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for FileName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl From<&str> for FileName {
    fn from(name: &str) -> Self {
        Self(name.to_owned())
    }
}

impl From<String> for FileName {
    fn from(name: String) -> Self {
        Self(name)
    }
}

impl AsRef<Path> for FileName {
    /// A bare name is a valid single-component path, which is what makes
    /// `panel.path.join(name)` the one correct way to turn a mark into a path.
    fn as_ref(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl std::fmt::Display for FileName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What fills the frame behind any modal overlay.
///
/// Split out of [`Overlay`] because hiding the panels is not a dialog: this
/// says *what is on screen*, while `Overlay` says *which dialog is open*. As
/// one enum the two were mutually exclusive, so the command line could not be
/// used while the console showed — and letting you keep working with the
/// output in front of you is the entire point of Far Manager's `Ctrl+O`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ViewMode {
    /// The dual-panel view.
    #[default]
    Panels,
    /// `Ctrl+O`: the panels are lifted off and the console transcript shows.
    Console,
}

impl ViewMode {
    /// The other mode — what `Ctrl+O` switches to.
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Panels => Self::Console,
            Self::Console => Self::Panels,
        }
    }
}

/// The ten bookmark slots, indexed by [`BookmarkSlot`].
///
/// A fixed-size array rather than a `Vec`: `BookmarkSlot` already proves the
/// index is 0..=9, so with a matching length every lookup is total. That is
/// what removed two identical `if idx >= bookmarks.len()` guards from the
/// dispatcher, and the "defensive check used when the Vec length may not be
/// MAX+1" comment they were written against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bookmarks([Option<PathBuf>; BookmarkSlot::COUNT]);

impl Default for Bookmarks {
    fn default() -> Self {
        Self(std::array::from_fn(|_| None))
    }
}

impl Bookmarks {
    /// The path saved in `slot`, if any. Cannot fail: the slot proves itself.
    #[must_use]
    pub fn get(&self, slot: BookmarkSlot) -> Option<&Path> {
        self.0[slot.as_usize()].as_deref()
    }

    pub fn set(&mut self, slot: BookmarkSlot, path: PathBuf) {
        self.0[slot.as_usize()] = Some(path);
    }

    pub fn clear(&mut self, slot: BookmarkSlot) {
        self.0[slot.as_usize()] = None;
    }

    /// Every slot in order, for the completer's `bookmark go` candidates.
    #[must_use]
    pub fn slots(&self) -> &[Option<PathBuf>] {
        &self.0
    }
}

/// Whether the event loop should keep running.
///
/// One representation, replacing `should_quit: bool` *beside* a `Flow`
/// enum returned by the key dispatcher. The loop condition read the flag while
/// the dispatch site matched the enum, so "are we leaving" had two answers
/// that had to be kept in step by hand — and the overlay layer could only
/// reach one of them, which is why the flag existed at all.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Lifecycle {
    #[default]
    Running,
    /// Something asked to leave: F10, Ctrl+C, or the `quit` / `exit` command.
    Quitting,
}

impl Lifecycle {
    #[must_use]
    pub const fn is_running(self) -> bool {
        matches!(self, Self::Running)
    }
}

/// Which panel is currently active: left or right
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PanelSide {
    Left,
    Right,
}

/// Domain-specific type for a bookmark slot (0-9).
/// Replaces raw `usize` / magic numbers for bookmark operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct BookmarkSlot(u8);

impl BookmarkSlot {
    /// Maximum valid slot (inclusive).
    pub const MAX: u8 = 9;
    /// Total number of bookmark slots available.
    pub const COUNT: usize = (Self::MAX as usize) + 1;

    /// Create a new slot. Returns error for values outside 0..=9.
    pub const fn new(n: u8) -> Result<Self, &'static str> {
        if n <= Self::MAX {
            Ok(Self(n))
        } else {
            Err("bookmark slot must be 0-9")
        }
    }

    /// Create from usize (common in parsing). Returns error if out of range.
    pub const fn from_usize(n: usize) -> Result<Self, &'static str> {
        if n <= Self::MAX as usize {
            Ok(Self(n as u8))
        } else {
            Err("bookmark slot must be 0-9")
        }
    }

    pub const fn as_u8(self) -> u8 {
        self.0
    }

    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl std::fmt::Display for BookmarkSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Main application state: two panels, overlays, bookmarks, etc.
#[derive(Clone)]
pub struct App {
    pub left_panel: Panel,
    pub right_panel: Panel,
    pub active_panel: PanelSide,

    /// What is drawn behind the overlays: the panels, or the console
    /// transcript that `Ctrl+O` reveals. Orthogonal to `overlay` on purpose —
    /// see [`ViewMode`].
    pub view: ViewMode,

    /// Single source of truth for all modal / overlay state.
    /// Using one enum enforces mutual exclusivity and makes illegal states
    /// (two overlays open at once) impossible at compile time.
    pub overlay: Overlay,

    pub bookmarks: Bookmarks,
    pub lifecycle: Lifecycle,
    pub status: crate::status::Status,
    pub quick_search: QuickSearch,

    /// Executed `:` commands, session-wide, persisted per the durability gate.
    pub history: crate::history::History,

    /// Far-style console transcript: every executed command and what it
    /// printed, drawn as the layer *behind* the panels and revealed by
    /// `Ctrl+O`. In memory only — unlike `history`, nothing here is ever
    /// written to disk, so a token in a shell command dies with the process.
    pub console: ConsoleLog,
}

impl App {
    /// Build session state only — no disk I/O, ever, in any build.
    ///
    /// `history` starts empty (`History::default()`, `path: None`, `save()` a
    /// permanent no-op — see `history.rs`). The real on-disk history is loaded
    /// exactly once at the production entry point (`main_impl` in `lib.rs`),
    /// never here: keeping `App::new()` pure means no test — unit or
    /// integration, present or future — can pollute the user's real
    /// `~/.local/share/ffm/history` merely by constructing an `App`.
    pub fn new() -> io::Result<Self> {
        let current_dir = std::env::current_dir()?;
        Ok(Self {
            left_panel: Panel::new(current_dir.clone())?,
            right_panel: Panel::new(current_dir)?,
            active_panel: PanelSide::Left,
            view: ViewMode::default(),
            overlay: Overlay::default(),
            bookmarks: Bookmarks::default(),
            lifecycle: Lifecycle::default(),
            status: crate::status::Status::KeyHints,
            quick_search: QuickSearch::default(),
            history: crate::history::History::default(),
            console: ConsoleLog::default(),
        })
    }

    pub const fn get_active_panel(&mut self) -> &mut Panel {
        match self.active_panel {
            PanelSide::Left => &mut self.left_panel,
            PanelSide::Right => &mut self.right_panel,
        }
    }

    pub const fn get_active_panel_ref(&self) -> &Panel {
        match self.active_panel {
            PanelSide::Left => &self.left_panel,
            PanelSide::Right => &self.right_panel,
        }
    }

    pub const fn get_inactive_panel(&mut self) -> &mut Panel {
        match self.active_panel {
            PanelSide::Left => &mut self.right_panel,
            PanelSide::Right => &mut self.left_panel,
        }
    }

    pub const fn get_inactive_panel_ref(&self) -> &Panel {
        match self.active_panel {
            PanelSide::Left => &self.right_panel,
            PanelSide::Right => &self.left_panel,
        }
    }

    // === Overlay helpers (new enum-based API) ===

    pub const fn is_overlay_active(&self) -> bool {
        self.overlay.is_active()
    }

    pub fn close_overlay(&mut self) {
        self.overlay.close();
    }

    /// Returns a reference to the current overlay.
    pub const fn get_overlay(&self) -> &Overlay {
        &self.overlay
    }

    /// Returns a mutable reference to the current overlay.
    pub const fn get_overlay_mut(&mut self) -> &mut Overlay {
        &mut self.overlay
    }

    /// Convenience: returns the active Viewer if the overlay is Viewer.
    pub const fn get_viewer(&self) -> Option<&Viewer> {
        if let Overlay::Viewer(v) = &self.overlay {
            Some(v)
        } else {
            None
        }
    }

    pub const fn get_viewer_mut(&mut self) -> Option<&mut Viewer> {
        if let Overlay::Viewer(v) = &mut self.overlay {
            Some(v)
        } else {
            None
        }
    }

    /// Convenience: returns the active Editor if the overlay is Editor.
    pub const fn get_editor(&self) -> Option<&Editor> {
        if let Overlay::Editor(e) = &self.overlay {
            Some(e)
        } else {
            None
        }
    }

    pub const fn get_editor_mut(&mut self) -> Option<&mut Editor> {
        if let Overlay::Editor(e) = &mut self.overlay {
            Some(e)
        } else {
            None
        }
    }

    pub fn get_command_line(&self) -> Option<&str> {
        if let Overlay::CommandLine(state) = &self.overlay {
            Some(state.line.text())
        } else {
            None
        }
    }

    pub fn get_new_file_input(&self) -> Option<&str> {
        if let Overlay::NewFileInput(s) = &self.overlay {
            Some(s)
        } else {
            None
        }
    }

    pub const fn get_delete_confirm(&self) -> Option<&Vec<PathBuf>> {
        if let Overlay::DeleteConfirm(files) = &self.overlay {
            Some(files)
        } else {
            None
        }
    }

    pub fn take_delete_confirm(&mut self) -> Option<Vec<PathBuf>> {
        match std::mem::take(&mut self.overlay) {
            Overlay::DeleteConfirm(paths) => Some(paths),
            other => {
                self.overlay = other;
                None
            }
        }
    }

    pub const fn get_new_file_input_mut(&mut self) -> Option<&mut String> {
        if let Overlay::NewFileInput(s) = &mut self.overlay {
            Some(s)
        } else {
            None
        }
    }

    /// The editable line, when the command-line overlay is open.
    ///
    /// Returns the [`LineEditor`], never a `&mut String`: handing out the raw
    /// buffer would let a caller shorten the text under a caret that then
    /// points past its end, which is exactly the invariant `LineEditor` exists
    /// to hold.
    pub const fn get_command_line_mut(&mut self) -> Option<&mut LineEditor> {
        if let Overlay::CommandLine(state) = &mut self.overlay {
            Some(&mut state.line)
        } else {
            None
        }
    }

    /// Replaces the current overlay with a new one.
    /// Only one overlay can be active at a time (enforced by the type system).
    pub fn set_overlay(&mut self, overlay: Overlay) {
        self.overlay = overlay;
    }

    pub const fn switch_panel(&mut self) {
        self.active_panel = match self.active_panel {
            PanelSide::Left => PanelSide::Right,
            PanelSide::Right => PanelSide::Left,
        };
    }

    /// Swap the contents (paths) of left and right panels (Ctrl+U)
    pub const fn swap_panels(&mut self) {
        std::mem::swap(&mut self.left_panel, &mut self.right_panel);
    }

    /// Get available drives (Windows) or mount points (Unix)
    #[cfg(windows)]
    pub fn get_drives() -> Vec<PathBuf> {
        let mask = unsafe { GetLogicalDrives() };
        windows_drive_paths_from_mask(mask)
    }

    #[cfg(not(windows))]
    pub fn get_drives() -> Vec<PathBuf> {
        vec![PathBuf::from("/")]
    }

    /// Get all available "drives" including local drives and AWS S3 profiles
    pub fn get_all_drives() -> Vec<DriveEntry> {
        #[cfg_attr(not(feature = "s3"), allow(unused_mut))]
        let mut entries: Vec<DriveEntry> = Self::get_drives()
            .into_iter()
            .map(DriveEntry::Local)
            .collect();

        #[cfg(feature = "s3")]
        for profile in crate::s3::list_aws_profiles() {
            entries.push(DriveEntry::S3Profile(profile.name));
        }

        entries
    }
}

#[cfg(windows)]
fn windows_drive_paths_from_mask(mask: u32) -> Vec<PathBuf> {
    let mut drives = Vec::new();
    for bit in 0..26 {
        if (mask & (1 << bit)) != 0 {
            let drive_letter = (b'A' + bit as u8) as char;
            drives.push(PathBuf::from(format!("{drive_letter}:\\")));
        }
    }
    drives
}

/// Represents a drive entry (local or S3)
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum DriveEntry {
    Local(PathBuf),
    S3Profile(String),
}

impl DriveEntry {
    #[must_use]
    pub fn display(&self) -> String {
        match self {
            Self::Local(path) => path.display().to_string(),
            Self::S3Profile(name) => format!("S3: {name}"),
        }
    }
}

/// Represents the currently active overlay / modal dialog.
///
/// This enum replaces a collection of boolean and Option fields
/// (`show_help`, `viewer`, `command_line`, `show_drives`, `delete_confirm`, etc.)
/// and enforces that only one overlay can be active at a time.
#[derive(Clone, Default)]
pub enum Overlay {
    /// No overlay is active (normal dual-panel view)
    #[default]
    None,

    /// F1 help screen
    Help,

    /// F3 file viewer (also reused for MCP status via F9 / :mcp)
    Viewer(Viewer),

    /// F4 built-in text editor
    Editor(Editor),

    /// Alt+F1 / Alt+F2 drive selection
    DriveSelection {
        side: PanelSide,
        selected_index: usize,
    },

    /// Shift+F4 "create new file"
    NewFileInput(String),

    /// Command line (pressed ':')
    CommandLine(CommandLine),

    /// F8 delete confirmation dialog
    DeleteConfirm(Vec<PathBuf>),
}

impl Overlay {
    /// Returns true if any modal overlay is currently active.
    pub const fn is_active(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Close the current overlay (return to normal view).
    pub fn close(&mut self) {
        *self = Self::None;
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_editor_save;
#[cfg(test)]
mod tests_overlay_accessors;
#[cfg(test)]
mod tests_panel_edge_cases;
#[cfg(test)]
mod tests_panel_enter_dotdot;
#[cfg(test)]
mod tests_panel_enter_marks;
#[cfg(test)]
mod tests_panel_navigation;
#[cfg(test)]
mod tests_panel_navigation_invariant;
#[cfg(test)]
mod tests_panel_paging;
#[cfg(test)]
mod tests_panel_sorting;
#[cfg(test)]
mod tests_panel_symlinks;
