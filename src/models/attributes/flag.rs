//! Purpose: What each checkbox of the attributes dialog stands for: a mode
//! bit on Unix, an attribute flag on Windows, and the rows they are drawn in.

/// What one checkbox stands for: a mode bit on Unix, an attribute flag on
/// Windows.
///
/// Both platforms' flags are named so the Windows rows can be tested on a
/// Unix host; [`Flag::GROUPS`] picks the platform's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    OwnerRead,
    OwnerWrite,
    OwnerExecute,
    GroupRead,
    GroupWrite,
    GroupExecute,
    OtherRead,
    OtherWrite,
    OtherExecute,
    SetUid,
    SetGid,
    Sticky,
    ReadOnly,
    Hidden,
    System,
    Archive,
}

/// One row of checkboxes: its caption and the flags it holds, left to right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlagGroup {
    pub caption: &'static str,
    pub flags: &'static [Flag],
}

const UNIX_GROUPS: &[FlagGroup] = &[
    FlagGroup {
        caption: "Owner",
        flags: &[Flag::OwnerRead, Flag::OwnerWrite, Flag::OwnerExecute],
    },
    FlagGroup {
        caption: "Group",
        flags: &[Flag::GroupRead, Flag::GroupWrite, Flag::GroupExecute],
    },
    FlagGroup {
        caption: "Other",
        flags: &[Flag::OtherRead, Flag::OtherWrite, Flag::OtherExecute],
    },
    FlagGroup {
        caption: "Special",
        flags: &[Flag::SetUid, Flag::SetGid, Flag::Sticky],
    },
];

const WINDOWS_GROUPS: &[FlagGroup] = &[FlagGroup {
    caption: "Flags",
    flags: &[Flag::ReadOnly, Flag::Hidden, Flag::System, Flag::Archive],
}];

impl Flag {
    /// The platform's checkboxes, in the rows the dialog draws them.
    pub const GROUPS: &'static [FlagGroup] = if cfg!(windows) {
        WINDOWS_GROUPS
    } else {
        UNIX_GROUPS
    };

    /// The bit in the entry's attribute word — `st_mode` or the Windows
    /// attribute flags, the same word `Attributes` renders.
    ///
    /// The two platforms' low bits coincide numerically (Windows' `0x20`
    /// Archive is Unix's `0o040`), which is why some arms are shared: the
    /// same number, meaning a different thing on a different disk.
    #[must_use]
    pub const fn bit(self) -> u32 {
        match self {
            Self::OwnerRead => 0o400,
            Self::OwnerWrite => 0o200,
            Self::OwnerExecute => 0o100,
            Self::GroupRead | Self::Archive => 0o040,
            Self::GroupWrite => 0o020,
            Self::GroupExecute => 0o010,
            Self::OtherRead | Self::System => 0o004,
            Self::OtherWrite | Self::Hidden => 0o002,
            Self::OtherExecute | Self::ReadOnly => 0o001,
            Self::SetUid => 0o4000,
            Self::SetGid => 0o2000,
            Self::Sticky => 0o1000,
        }
    }

    /// The word beside the box.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::OwnerRead | Self::GroupRead | Self::OtherRead => "Read",
            Self::OwnerWrite | Self::GroupWrite | Self::OtherWrite => "Write",
            Self::OwnerExecute | Self::GroupExecute | Self::OtherExecute => "Execute",
            Self::SetUid => "Set UID",
            Self::SetGid => "Set GID",
            Self::Sticky => "Sticky",
            Self::ReadOnly => "Read-only",
            Self::Hidden => "Hidden",
            Self::System => "System",
            Self::Archive => "Archive",
        }
    }
}
