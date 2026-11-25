//! Purpose: How a panel orders its rows — Far's sort modes, their names, the
//! header letter, and the `:sort` words that pick them.

/// How the panel orders its rows — Far's `Ctrl+F3` … `Ctrl+F11`.
///
/// `Unsorted` is the directory's own order, which on most filesystems means
/// creation order and on none of them means anything the user chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum SortMode {
    #[default]
    Name,
    Extension,
    Size,
    ModificationTime,
    Unsorted,
    CreationTime,
    AccessTime,
    Descriptions,
    Owner,
}

impl SortMode {
    /// In the order of Far's Sort modes menu.
    pub const ALL: [Self; 9] = [
        Self::Name,
        Self::Extension,
        Self::ModificationTime,
        Self::Size,
        Self::Unsorted,
        Self::CreationTime,
        Self::AccessTime,
        Self::Descriptions,
        Self::Owner,
    ];

    /// The legacy three-stop cycle (name, extension, size) kept for its
    /// callers; the Far-only modes walk the rest of the menu back to `Name`.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Name => Self::Extension,
            Self::Extension => Self::Size,
            Self::Size | Self::Owner => Self::Name,
            Self::ModificationTime => Self::Unsorted,
            Self::Unsorted => Self::CreationTime,
            Self::CreationTime => Self::AccessTime,
            Self::AccessTime => Self::Descriptions,
            Self::Descriptions => Self::Owner,
        }
    }

    /// The name the status line and the sort menu show.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Extension => "Ext",
            Self::Size => "Size",
            Self::ModificationTime => "Modification time",
            Self::Unsorted => "Unsorted",
            Self::CreationTime => "Creation time",
            Self::AccessTime => "Access time",
            Self::Descriptions => "Descriptions",
            Self::Owner => "Owner",
        }
    }

    /// The letter Far puts in the panel header's corner — the only place the
    /// panel says how it is ordered.
    #[must_use]
    pub const fn letter(self) -> char {
        match self {
            Self::Name => 'n',
            Self::Extension => 'e',
            Self::Size => 's',
            Self::ModificationTime => 'm',
            Self::Unsorted => 'u',
            Self::CreationTime => 'c',
            Self::AccessTime => 'a',
            Self::Descriptions => 'd',
            Self::Owner => 'o',
        }
    }

    /// The `:sort` argument for the mode, and its accepted spellings.
    const WORDS: [(Self, &'static [&'static str]); 9] = [
        (Self::Name, &["name"]),
        (Self::Extension, &["ext", "extension"]),
        (Self::Size, &["size"]),
        (Self::ModificationTime, &["mtime", "modified"]),
        (Self::Unsorted, &["unsorted"]),
        (Self::CreationTime, &["ctime", "created"]),
        (Self::AccessTime, &["atime", "accessed"]),
        (Self::Descriptions, &["desc", "descriptions"]),
        (Self::Owner, &["owner"]),
    ];

    /// The mode a `:sort` word names, or `None` for a word that is not one.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::WORDS
            .iter()
            .find(|(_, words)| words.contains(&word))
            .map(|(mode, _)| *mode)
    }

    /// Every spelling `:sort` accepts, for the usage line.
    pub const USAGE: &'static str =
        "sort expects name|ext|size|mtime|unsorted|ctime|atime|desc|owner";
}
