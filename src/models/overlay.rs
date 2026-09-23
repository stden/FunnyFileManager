//! Purpose: The modal overlays, and the small types the dialogs are made of.
//!
//! Split out of `models.rs` when it passed the 1000-line cap
//! `tests/file_line_limit_test.rs` enforces. The boundary is not arbitrary:
//! everything here is about something that takes over the keyboard, and
//! `models.rs` keeps what is always on screen.

use std::path::PathBuf;

use super::{
    ApplyPrompt, ArchiveMenu, ArchivePrompt, AssociationList, AttributesDialog, CommandLine,
    DescribePrompt, DeviceList, DriveRow, Editor, ExtractPrompt, FileName, FilterPrompt, FindFile,
    FolderTree, HistoryList, PanelSide, ScreenList, SettingsDialog, ShortcutList, SortMenu,
    TaskList, UserMenu, Viewer,
};

/// Which button a two-answer dialog has focus on.
///
/// Two named answers rather than a `bool` or an index: `Confirm::No` says what
/// it means at the call site, where `false` would need the reader to remember
/// which way round the dialog was built.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Confirm {
    /// The default, and what Enter starts on.
    #[default]
    Yes,
    No,
}

impl Confirm {
    /// The other answer — what Left, Right and Tab all move to, there being
    /// only two.
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Yes => Self::No,
            Self::No => Self::Yes,
        }
    }

    #[must_use]
    pub const fn is_yes(self) -> bool {
        matches!(self, Self::Yes)
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

    /// Windows Subsystem for Linux distributions and maintenance commands.
    Wsl(Box<super::WslDialog>),
    PathEditor(Box<crate::path_env::PathEditor>),

    /// F3 file viewer (also reused for MCP status via F11 / :mcp)
    Viewer(Viewer),

    /// F4 built-in text editor
    Editor(Editor),

    /// Alt+F1 / Alt+F2 drive selection
    DriveSelection {
        side: PanelSide,
        selected_index: usize,
        /// Built once, when the dialog opens. A row carries its filesystem's
        /// size, and the event loop redraws after every key: rows rebuilt in
        /// the renderer would cost a `statvfs` per drive per frame.
        rows: Vec<DriveRow>,
    },

    /// Shift+F4 "create new file"
    NewFileInput(String),

    /// Command line (pressed ':')
    CommandLine(CommandLine),

    /// F8 delete confirmation dialog
    DeleteConfirm(Vec<PathBuf>),

    /// F10 "do you want to quit?", carrying which button has the cursor.
    QuitConfirm(Confirm),

    /// A dialog asking for a name: Shift+F6 rename, F7 make folder.
    NamePrompt(NamePrompt),

    /// Ctrl+L: what the active directory contains, measured.
    Information(Box<crate::stats::DirectoryStats>),

    /// F9: the menu bar's open heading and where the cursor is in it.
    Menu(crate::menu::Menu),
    MenuDialog(Box<super::MenuDialog>),

    /// Gray + / Gray −: which files to add to or take out of the selection.
    MaskPrompt(MaskPrompt),

    /// F5: where to copy, and how. Boxed because it is much the largest
    /// variant, and an `Overlay` sits inside `App` whether a dialog is open
    /// or not.
    CopyPrompt(Box<crate::copy::CopyPrompt>),

    /// Alt+Del: which files to wipe, asked the way F8 asks.
    WipeConfirm(Vec<PathBuf>),
    /// Shift+F1: Add to archive.
    ArchivePrompt(Box<ArchivePrompt>),
    /// Shift+F2: Extract files.
    ExtractPrompt(Box<ExtractPrompt>),
    /// Shift+F3: Archive commands.
    ArchiveCommands(ArchiveMenu),
    /// Ctrl+A: File attributes.
    Attributes(Box<AttributesDialog>),
    /// Ctrl+G: Apply command.
    ApplyCommand(ApplyPrompt),
    /// Ctrl+Z: Describe files.
    Describe(DescribePrompt),
    /// Alt+F7: Find file.
    FindFile(Box<FindFile>),
    /// Alt+F8: the command history as a list.
    CommandHistory(HistoryList),
    /// Alt+F10: Find folder — the tree as a dialog.
    FindFolder(Box<FolderTree>),
    /// Alt+F11: File view history.
    ViewHistory(HistoryList),
    /// Alt+F12: Folders history.
    FolderHistory(HistoryList),
    /// F2: the user menu.
    UserMenu(Box<UserMenu>),
    /// Commands menu: File associations.
    Associations(Box<AssociationList>),
    /// Commands menu: Folder shortcuts.
    FolderShortcuts(ShortcutList),
    /// Ctrl+I: File panel filter.
    Filter(FilterPrompt),
    /// F12: Screens list.
    Screens(ScreenList),
    /// Ctrl+W: Task list.
    Tasks(Box<TaskList>),
    /// Commands menu: Hotplug devices list.
    Hotplug(Box<DeviceList>),
    /// Ctrl+F12: Sort modes.
    SortModes(SortMenu),
    /// Options menu: one settings page.
    Settings(Box<SettingsDialog>),

    /// `:cleanup` — the disk cleanup checklist.
    DiskCleanup(Box<super::DiskCleanupDialog>),

    /// Enter on a `.kdbx` file: the unlock prompt, then the entry browser.
    /// Wrapped in [`KdbxOverlay`](crate::models::KdbxOverlay) because this
    /// enum derives `Clone` and a session's master key must not be duplicated
    /// — the wrapper's `Clone` hands back the same vault, locked. This enum
    /// must stay without `Debug` for the same reason.
    #[cfg(feature = "keepass")]
    Kdbx(crate::models::KdbxOverlay),
}

/// What a [`NamePrompt`] is asking for, which is the only thing that differs
/// between the two dialogs — the typing, the editing and the cancelling are
/// the same, so they are written once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamePurpose {
    /// Shift+F6: the entry keeps its place and changes its name.
    Rename,
    /// F7: a new directory in the active panel.
    MakeFolder,
    /// Alt+F6: a link to the entry under the cursor.
    Link,
}

impl NamePurpose {
    /// The dialog's title.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Rename => "Rename",
            Self::MakeFolder => "Make folder",
            Self::Link => "Link",
        }
    }

    /// Whether the dialog asks what kind of link to make. Rename does not:
    /// it is Far's plain box, one field and nothing else.
    #[must_use]
    pub const fn offers_links(self) -> bool {
        matches!(self, Self::MakeFolder | Self::Link)
    }

    /// The link kinds this dialog's drop-down steps through.
    ///
    /// Two lists rather than one: no kernel this runs on will hard-link a
    /// directory, so F7 must not offer it, while Alt+F6 on a file has no use
    /// for "none" — a link that is not a link is nothing at all.
    #[must_use]
    pub const fn link_choices(self) -> &'static [LinkType] {
        match self {
            Self::Link => &LinkType::LINK_CHOICES,
            _ => &LinkType::FOLDER_CHOICES,
        }
    }
}

/// Which of Far's two group keys put the mask dialog up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaskPurpose {
    /// Gray +
    Select,
    /// Gray −
    Unselect,
}

impl MaskPurpose {
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Unselect => "Deselect",
        }
    }

    /// Whether the matching rows join the selection or leave it.
    #[must_use]
    pub const fn selects(self) -> bool {
        matches!(self, Self::Select)
    }
}

/// Gray + / Gray −: which files, given as a mask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaskPrompt {
    purpose: MaskPurpose,
    mask: String,
    /// The cursor is on Cancel rather than on OK.
    on_cancel: bool,
}

impl MaskPrompt {
    /// Open the dialog, offering `suggestion` as the mask.
    ///
    /// Far offers the extension of the file under the cursor, because the
    /// commonest thing anyone does with this key is "all the other ones like
    /// this". A field that opens on `*` makes them type it every time.
    #[must_use]
    pub fn new(purpose: MaskPurpose, suggestion: String) -> Self {
        Self {
            purpose,
            mask: suggestion,
            on_cancel: false,
        }
    }

    /// The mask Far would offer for `name`: its extension, or everything.
    #[must_use]
    pub fn suggest(name: Option<&str>) -> String {
        name.and_then(|name| name.rsplit_once('.'))
            // A leading dot starts a name, not an extension: `.gitignore` has
            // no extension, and offering `*.gitignore` would be nonsense.
            .filter(|(stem, extension)| !stem.is_empty() && !extension.is_empty())
            .map_or_else(|| "*".to_owned(), |(_, extension)| format!("*.{extension}"))
    }

    #[must_use]
    pub const fn purpose(&self) -> MaskPurpose {
        self.purpose
    }

    #[must_use]
    pub fn mask(&self) -> &str {
        &self.mask
    }

    #[must_use]
    pub const fn on_cancel(&self) -> bool {
        self.on_cancel
    }

    pub const fn focus_cancel(&mut self, cancel: bool) {
        self.on_cancel = cancel;
    }

    pub const fn toggle_button(&mut self) {
        self.on_cancel = !self.on_cancel;
    }

    pub fn push(&mut self, ch: char) {
        self.mask.push(ch);
    }

    pub fn backspace(&mut self) {
        self.mask.pop();
    }
}

/// What F7 makes — a directory, or a link that stands for one — and what
/// Alt+F6 makes: a link to the entry under the cursor.
///
/// Far's own list also offers junctions. A junction is a Windows idea, so it
/// would be a control that always fails, which is worse than not offering it
/// at all. Which of these a dialog offers is [`NamePurpose::link_choices`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinkType {
    /// A plain directory.
    #[default]
    None,
    Symbolic,
    /// A second name for the same file. Files only: see
    /// [`NamePurpose::link_choices`].
    Hard,
}

impl LinkType {
    /// What F7 steps through, in order.
    pub const FOLDER_CHOICES: [Self; 2] = [Self::None, Self::Symbolic];
    /// What Alt+F6 steps through, in order.
    pub const LINK_CHOICES: [Self; 2] = [Self::Symbolic, Self::Hard];

    /// What the closed drop-down shows.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Symbolic => "symbolic link",
            Self::Hard => "hard link",
        }
    }

    /// Whether the Target field applies. A plain directory has no target, so
    /// the field is greyed rather than taking text that would be ignored.
    #[must_use]
    pub const fn needs_target(self) -> bool {
        matches!(self, Self::Symbolic | Self::Hard)
    }
}

/// Which control in the name dialog has the cursor.
///
/// An enum rather than an index, for the reason [`crate::copy::CopyField`] is
/// one: the tab order is stated once, and a control added in the middle cannot
/// silently renumber the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NameField {
    #[default]
    Name,
    Link,
    Target,
    Multiple,
    Ok,
    Cancel,
}

impl NameField {
    /// The tab order, top to bottom and then along the buttons.
    pub(crate) const ORDER: [Self; 6] = [
        Self::Name,
        Self::Link,
        Self::Target,
        Self::Multiple,
        Self::Ok,
        Self::Cancel,
    ];

    #[must_use]
    pub fn next(self) -> Self {
        crate::cycle::step(&Self::ORDER, self, true)
    }

    #[must_use]
    pub fn previous(self) -> Self {
        crate::cycle::step(&Self::ORDER, self, false)
    }

    /// Whether typing lands here — which is also what decides whether the
    /// space bar is a space or a toggle.
    #[must_use]
    pub const fn is_text(self) -> bool {
        matches!(self, Self::Name | Self::Target)
    }
}

/// A dialog asking for a name, and the name being typed into it.
///
/// Shift+F6 uses one field of it and F7 uses all of them: Far's Make folder
/// dialog also asks what kind of link to make, what it should point at, and
/// whether the name field is a list. Both are the same dialog because the
/// typing, the editing and the cancelling are the same.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamePrompt {
    purpose: NamePurpose,
    /// What the entry is called now, for a rename. Empty for a new folder,
    /// which is not renaming anything.
    original: FileName,
    name: String,
    link: LinkType,
    /// What a link points at. Free text, and a path rather than a name: unlike
    /// the name field, separators are exactly what belongs here.
    link_target: String,
    /// Far's "Process multiple names": the name field is then a `;`-separated
    /// list, and every part of it is created.
    multiple: bool,
    field: NameField,
}

impl NamePrompt {
    /// A rename, opening on the current name: a rename is usually a small
    /// change to a name rather than a fresh one.
    #[must_use]
    pub fn rename(original: FileName) -> Self {
        Self {
            purpose: NamePurpose::Rename,
            name: original.as_str().to_owned(),
            original,
            link: LinkType::None,
            link_target: String::new(),
            multiple: false,
            field: NameField::Name,
        }
    }

    /// A new folder, opening empty.
    #[must_use]
    pub fn make_folder() -> Self {
        Self {
            purpose: NamePurpose::MakeFolder,
            original: FileName::new(String::new()),
            name: String::new(),
            link: LinkType::None,
            link_target: String::new(),
            multiple: false,
            field: NameField::Name,
        }
    }

    /// A link to `target`, opening on `name` and on a symbolic link, which is
    /// the kind that works for everything.
    ///
    /// Alt+F6 offers the other panel's directory joined with the target's
    /// own name: Far puts a link there unless told otherwise, and showing the
    /// whole path is what lets the user tell it otherwise.
    #[must_use]
    pub fn link_to(target: &std::path::Path, name: String) -> Self {
        Self {
            purpose: NamePurpose::Link,
            original: FileName::new(String::new()),
            name,
            link: LinkType::Symbolic,
            link_target: target.display().to_string(),
            multiple: false,
            field: NameField::Name,
        }
    }

    #[must_use]
    pub const fn purpose(&self) -> NamePurpose {
        self.purpose
    }

    #[must_use]
    pub fn original(&self) -> &str {
        self.original.as_str()
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn link(&self) -> LinkType {
        self.link
    }

    #[must_use]
    pub fn link_target(&self) -> &str {
        self.link_target.trim()
    }

    #[must_use]
    pub const fn multiple(&self) -> bool {
        self.multiple
    }

    #[must_use]
    pub const fn field(&self) -> NameField {
        self.field
    }

    /// Whether this dialog offers that control at all.
    ///
    /// Shift+F6 is Far's plain Rename box: it asks for a name and nothing
    /// else. The link controls belong to F7, and the Target field only once
    /// there is a link for it to describe — greyed otherwise, so the dialog
    /// keeps its shape as the drop-down changes.
    #[must_use]
    pub const fn allows(&self, field: NameField) -> bool {
        match field {
            NameField::Name | NameField::Ok | NameField::Cancel => true,
            NameField::Link => self.purpose.offers_links(),
            NameField::Multiple => matches!(self.purpose, NamePurpose::MakeFolder),
            NameField::Target => self.purpose.offers_links() && self.link.needs_target(),
        }
    }

    /// Move to the next control the dialog actually offers, skipping the
    /// greyed ones: stopping on a control that does nothing would undo the
    /// point of greying it.
    pub fn advance(&mut self, forward: bool) {
        for _ in 0..NameField::ORDER.len() {
            self.field = if forward {
                self.field.next()
            } else {
                self.field.previous()
            };
            if self.allows(self.field) {
                return;
            }
        }
    }

    /// Put the cursor on `field`, if the dialog offers it. What a click means.
    pub const fn focus(&mut self, field: NameField) {
        if self.allows(field) {
            self.field = field;
        }
    }

    /// Toggle the checkbox under the cursor. Does nothing elsewhere, so the
    /// space bar stays a space in the two text fields.
    pub const fn toggle(&mut self) {
        if matches!(self.field, NameField::Multiple) && self.allows(NameField::Multiple) {
            self.multiple = !self.multiple;
        }
    }

    /// Left/Right on the drop-down. Elsewhere the arrows are not ours.
    pub fn cycle(&mut self, forward: bool) {
        if !matches!(self.field, NameField::Link) || !self.allows(NameField::Link) {
            return;
        }
        self.link = crate::cycle::step(self.purpose.link_choices(), self.link, forward);
        // The Target field may have just been greyed out from under the
        // cursor. It cannot be, in fact — the cursor is on the drop-down — but
        // the invariant is worth holding rather than reasoning about.
        if !self.allows(self.field) {
            self.field = NameField::Name;
        }
    }

    /// Which control the dialog draws on body row `row`.
    ///
    /// The row order lives here rather than only in the renderer, because the
    /// mouse needs it too: a click is only right if it reaches the control the
    /// user is pointing at.
    #[must_use]
    pub const fn field_at_row(&self, row: usize) -> Option<NameField> {
        // 0 is the caption, 2 is the rule under the name field.
        match (self.purpose, row) {
            (_, 1) => Some(NameField::Name),
            (purpose, 3) if purpose.offers_links() => Some(NameField::Link),
            (purpose, 4) if purpose.offers_links() => Some(NameField::Target),
            (NamePurpose::MakeFolder, 5) => Some(NameField::Multiple),
            _ => None,
        }
    }

    pub fn push(&mut self, ch: char) {
        match self.field {
            NameField::Name => self.name.push(ch),
            NameField::Target => self.link_target.push(ch),
            _ => {}
        }
    }

    pub fn backspace(&mut self) {
        match self.field {
            NameField::Name => self.name.pop(),
            NameField::Target => self.link_target.pop(),
            _ => None,
        };
    }

    /// A name this dialog may act on, or `None` when there is nothing to do:
    /// blank, or — for a rename — the name the entry already has. A path
    /// separator is refused too, because that would move the entry somewhere
    /// else rather than name it, and moving is what F6 is for.
    fn usable<'a>(&self, candidate: &'a str) -> Option<&'a str> {
        let trimmed = candidate.trim();
        if trimmed.is_empty() || trimmed.contains('/') || trimmed.contains('\\') {
            return None;
        }
        if self.purpose == NamePurpose::Rename && trimmed == self.original.as_str() {
            return None;
        }
        Some(trimmed)
    }

    /// The name to act on, for the dialogs that ask for exactly one.
    #[must_use]
    pub fn target(&self) -> Option<&str> {
        self.usable(&self.name)
    }

    /// Every name to create, in the order they were typed.
    ///
    /// One, unless "Process multiple names" is ticked — Far's way of making
    /// several folders from one dialog, where the field is a `;`-separated
    /// list. Unusable parts are dropped rather than failing the whole line: a
    /// trailing `;` is how the list is normally typed.
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        if !self.multiple {
            return self.target().into_iter().collect();
        }
        self.name
            .split(';')
            .filter_map(|part| self.usable(part))
            .collect()
    }
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
