//! Purpose: `Shift+F3`'s Archive commands menu — the five rows, their hotkeys, and the cursor that moves through them.

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
        let [.., last] = ArchiveCommand::ALL;
        ArchiveCommand::ALL
            .get(self.cursor)
            .copied()
            .unwrap_or(last)
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
