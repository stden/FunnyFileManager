//! Purpose: The three archive dialogs (`Shift+F1`, `Shift+F2`, `Shift+F3`), which share one model file because they share one set of archive operations.
//!
//! State only: the operations that fill it live in `ops`, and the keys that
//! move through it in `popup`.

use std::path::{Path, PathBuf};

/// The archive kinds the dialogs read: what `Shift+F2` unpacks, what the
/// `Shift+F3` rows accept, and what the captions list when nothing under the
/// cursor is one.
pub const ARCHIVE_SUFFIXES: [&str; 6] = [".tar", ".tar.gz", ".tgz", ".tar.xz", ".tar.bz2", ".zip"];

/// How an archive is packed: which tool runs and what the name ends in.
///
/// Three rather than Far's plugin list, because these are the three every
/// Linux, macOS and Windows box can be expected to read back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArchiveFormat {
    #[default]
    TarGz,
    Tar,
    Zip,
}

impl ArchiveFormat {
    /// Every format, in the order the radio shows them.
    pub const ALL: [Self; 3] = [Self::TarGz, Self::Tar, Self::Zip];

    /// What an archive of this format is named after the dot.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::TarGz => "tar.gz",
            Self::Tar => "tar",
            Self::Zip => "zip",
        }
    }

    /// The radio label, with its hotkey marked.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::TarGz => "tar.&gz",
            Self::Tar => "&tar",
            Self::Zip => "&zip",
        }
    }

    /// The program that writes this format.
    #[must_use]
    pub const fn tool(self) -> &'static str {
        match self {
            Self::TarGz | Self::Tar => "tar",
            Self::Zip => "zip",
        }
    }

    /// The format an existing archive's name says it has.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        let lower = name.to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|format| lower.ends_with(&format!(".{}", format.extension())))
    }

    /// Whether files can be added to an existing archive of this format:
    /// `tar -r` appends to a plain tar and `zip` updates in place, but a
    /// compressed tar is one stream that cannot be extended.
    #[must_use]
    pub const fn appendable(self) -> bool {
        matches!(self, Self::Tar | Self::Zip)
    }
}

/// Which control in the Add to archive dialog has the cursor.
///
/// An enum rather than an index so the tab order is stated once, here, and a
/// control added in the middle cannot silently renumber the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArchiveField {
    #[default]
    Name,
    Format,
    DeleteAfter,
    Add,
    Cancel,
}

impl ArchiveField {
    /// The tab order, top to bottom and then along the buttons.
    const ORDER: [Self; 5] = [
        Self::Name,
        Self::Format,
        Self::DeleteAfter,
        Self::Add,
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

    /// Whether this control is a checkbox, and so answers to the space bar.
    #[must_use]
    pub const fn is_checkbox(self) -> bool {
        matches!(self, Self::DeleteAfter)
    }
}

/// `Shift+F1`: what to call the archive, how to pack it, and whether the
/// originals go.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArchivePrompt {
    /// The archive to create, offered as `<name>.tar.gz` in the other panel.
    /// A relative name lands in the active panel's folder.
    pub name: String,
    pub format: ArchiveFormat,
    /// Far's "Delete files after archiving": the sources go once the archive
    /// is written, and only then.
    pub delete_after: bool,
    pub field: ArchiveField,
    /// The formats this machine can write, in radio order. `zip` is offered
    /// only where it is installed: a radio button that fails when pressed is
    /// worse than one that is not there.
    pub offered: Vec<ArchiveFormat>,
    /// What goes in, as names relative to the active panel's folder.
    pub sources: Vec<String>,
    /// `name` is an archive that already exists and the sources join it —
    /// the Archive commands row, rather than `Shift+F1`'s new archive.
    pub append: bool,
}

impl ArchivePrompt {
    /// Open the dialog offering `<stem>.<first format>` inside `folder`.
    #[must_use]
    pub fn new(
        sources: Vec<String>,
        stem: &str,
        folder: &Path,
        offered: Vec<ArchiveFormat>,
    ) -> Self {
        let format = offered.first().copied().unwrap_or_default();
        let name = folder
            .join(format!("{stem}.{}", format.extension()))
            .display()
            .to_string();
        Self {
            name,
            format,
            offered,
            sources,
            ..Self::default()
        }
    }

    /// Open the dialog on an existing archive the sources are to join. The
    /// radio offers the archive's own format and nothing else: turning it
    /// would rename an archive that is already there.
    #[must_use]
    pub fn appending(sources: Vec<String>, archive: &Path, format: ArchiveFormat) -> Self {
        Self {
            name: archive.display().to_string(),
            format,
            offered: vec![format],
            sources,
            append: true,
            ..Self::default()
        }
    }

    /// The name Far's archiver plugin would offer: one file's name without
    /// its extension, one directory's whole name, and for several the folder
    /// they are in — the one name that describes all of them.
    #[must_use]
    pub fn suggest_stem(sources: &[String], single_is_dir: bool, folder_name: &str) -> String {
        match sources {
            [one] if single_is_dir => one.clone(),
            [one] => one
                .rsplit_once('.')
                // A leading dot starts a name, not an extension.
                .filter(|(stem, _)| !stem.is_empty())
                .map_or_else(|| one.clone(), |(stem, _)| stem.to_owned()),
            _ => folder_name.to_owned(),
        }
    }

    /// The caption over the name field: the one thing being packed, or how
    /// many.
    #[must_use]
    pub fn caption(&self) -> String {
        match self.sources.as_slice() {
            [one] => format!("Add \"{one}\" to the archive:"),
            many => format!("Add {} items to the archive:", many.len()),
        }
    }

    /// Where the chosen format sits among the offered ones, for the radio.
    #[must_use]
    pub fn format_index(&self) -> usize {
        self.offered
            .iter()
            .position(|format| *format == self.format)
            .unwrap_or(0)
    }

    /// Tab / Shift+Tab: the next or previous control, wrapping.
    pub fn advance(&mut self, forward: bool) {
        self.field = if forward {
            self.field.next()
        } else {
            self.field.previous()
        };
    }

    /// Space on the checkbox. Does nothing elsewhere, so the space bar stays
    /// a space in the name field.
    pub fn toggle(&mut self) {
        if self.field.is_checkbox() {
            self.delete_after = !self.delete_after;
        }
    }

    /// Left/Right on the radio: the next offered format, and the name's
    /// extension follows it. The name and the format must agree, and making
    /// the user retype the extension is how they stop agreeing.
    pub fn cycle(&mut self, forward: bool) {
        if self.field != ArchiveField::Format {
            return;
        }
        let previous = self.format;
        self.format = crate::cycle::step(&self.offered, previous, forward);
        let old_suffix = format!(".{}", previous.extension());
        if let Some(stem) = self.name.strip_suffix(&old_suffix) {
            self.name = format!("{stem}.{}", self.format.extension());
        }
    }

    /// Whether the dialog can be accepted. An empty name cannot.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        !self.name.trim().is_empty()
    }
}

/// Which control of the Extract files dialog carries the cursor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExtractField {
    /// The destination field, where the dialog opens.
    #[default]
    Destination,
    /// The default button.
    Extract,
    Cancel,
}

impl ExtractField {
    /// Tab order, top to bottom.
    const ORDER: [Self; 3] = [Self::Destination, Self::Extract, Self::Cancel];
}

/// `Shift+F2`: where the selected archives unpack to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtractPrompt {
    /// The destination, offered as the other panel's folder. Free text: a
    /// relative path is taken from the active panel, and a folder that does
    /// not exist yet is created.
    pub destination: String,
    /// The selected entries that are archives, absolute. Empty when nothing
    /// selected is one — the dialog still opens and says so, because a key
    /// that visibly does nothing reads as a dead key.
    pub archives: Vec<PathBuf>,
    pub field: ExtractField,
}

impl ExtractPrompt {
    /// Open the dialog for `archives`, offering `destination`.
    #[must_use]
    pub fn new(archives: Vec<PathBuf>, destination: &Path) -> Self {
        Self {
            destination: destination.to_string_lossy().into_owned(),
            archives,
            field: ExtractField::default(),
        }
    }

    /// There is something to extract, so the Extract button applies.
    #[must_use]
    pub fn can_extract(&self) -> bool {
        !self.archives.is_empty()
    }

    /// The dialog's first line: what is going to be unpacked.
    #[must_use]
    pub fn caption(&self) -> String {
        match self.archives.as_slice() {
            [] => format!("No archive selected ({})", ARCHIVE_SUFFIXES.join(" ")),
            [one] => format!(
                "Extract \"{}\" to:",
                one.file_name().unwrap_or_default().to_string_lossy()
            ),
            many => format!("Extract {} archives to:", many.len()),
        }
    }

    #[must_use]
    pub fn on_cancel(&self) -> bool {
        self.field == ExtractField::Cancel
    }

    /// Move the cursor to the next control (or the previous one), skipping
    /// the Extract button while it is dimmed, as Far skips a disabled control.
    pub fn advance(&mut self, forward: bool) {
        self.field = crate::cycle::step(&ExtractField::ORDER, self.field, forward);
        if self.field == ExtractField::Extract && !self.can_extract() {
            self.field = crate::cycle::step(&ExtractField::ORDER, self.field, forward);
        }
    }

    pub fn push(&mut self, ch: char) {
        self.destination.push(ch);
    }

    pub fn backspace(&mut self) {
        self.destination.pop();
    }
}

/// The rows of the Archive commands menu, in the order they are drawn.
///
/// Far's `MultiArc` offers a dozen — comments, SFX stubs, recovery records,
/// locking — that belong to particular Windows archivers. What `tar` and
/// `unzip` can do on every platform is these five. The two extract rows are
/// Far's "Extract" with the destination already answered, since here the
/// other panel is where things go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveCommand {
    /// Read the archive end to end without unpacking it.
    Test,
    /// Show what it holds, in the viewer.
    List,
    /// Unpack beside the archive.
    ExtractHere,
    /// Unpack into the other panel's folder.
    ExtractToOtherPanel,
    /// Put the marked files into this archive.
    AddSelected,
}

impl ArchiveCommand {
    /// Every row, top down.
    pub const ALL: [Self; 5] = [
        Self::Test,
        Self::List,
        Self::ExtractHere,
        Self::ExtractToOtherPanel,
        Self::AddSelected,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Test => "Test archive",
            Self::List => "List contents",
            Self::ExtractHere => "Extract here",
            Self::ExtractToOtherPanel => "Extract to other panel",
            Self::AddSelected => "Add selected files to this archive",
        }
    }

    /// The letter that runs the row without moving the cursor to it.
    ///
    /// Lower case, so a comparison with what was typed needs no folding on the
    /// caller's side; the renderer capitalises it the way Far's menus do.
    #[must_use]
    pub const fn hotkey(self) -> char {
        match self {
            Self::Test => 't',
            Self::List => 'l',
            Self::ExtractHere => 'h',
            Self::ExtractToOtherPanel => 'o',
            Self::AddSelected => 'a',
        }
    }

    /// The row `letter` stands for, in either case.
    #[must_use]
    pub fn by_hotkey(letter: char) -> Option<Self> {
        let wanted = letter.to_ascii_lowercase();
        Self::ALL.into_iter().find(|row| row.hotkey() == wanted)
    }
}

/// `Shift+F3`: Far's archive commands as a list — test, list, extract, add.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArchiveMenu {
    /// Which command carries the cursor.
    pub cursor: usize,
}

impl ArchiveMenu {
    /// The row under the cursor.
    ///
    /// A cursor past the end names the last row rather than nothing: the
    /// state can be built with any number, and a menu with no row chosen would
    /// draw a cursor on nothing while Enter did nothing.
    #[must_use]
    pub fn command(&self) -> ArchiveCommand {
        let last = ArchiveCommand::ALL.len() - 1;
        ArchiveCommand::ALL[self.cursor.min(last)]
    }

    /// One row up, wrapping to the bottom at the top — Far's menu cursor.
    pub const fn move_up(&mut self) {
        self.cursor = if self.cursor == 0 {
            ArchiveCommand::ALL.len() - 1
        } else {
            self.cursor - 1
        };
    }

    /// One row down, wrapping to the top past the end.
    pub const fn move_down(&mut self) {
        self.cursor = (self.cursor + 1) % ArchiveCommand::ALL.len();
    }

    pub const fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub const fn move_end(&mut self) {
        self.cursor = ArchiveCommand::ALL.len() - 1;
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_extract;
