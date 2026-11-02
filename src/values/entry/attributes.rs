//! Purpose: An entry's attribute word — Unix mode bits or Windows attribute
//! flags — and its `ls -l` and Far letter renderings.

/// An entry's attribute word: the Unix mode (`st_mode`) or the Windows
/// attribute flags, whichever the listing read.
///
/// One newtype for both because the row stores exactly one `u32` either way
/// and only the rendering differs — `drwxr-xr-x` from mode bits, `RHSA`
/// letters from flags. Both renderers are public so the Windows one can be
/// tested on a Unix host; `Display` picks the platform's own.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Attributes(u32);

impl Attributes {
    const WINDOWS_LETTERS: [(u32, char); 4] = [(0x1, 'R'), (0x2, 'H'), (0x4, 'S'), (0x20, 'A')];
    const UNIX_TYPES: [(u32, char); 7] = [
        (0o140_000, 's'),
        (0o120_000, 'l'),
        (0o100_000, '-'),
        (0o060_000, 'b'),
        (0o040_000, 'd'),
        (0o020_000, 'c'),
        (0o010_000, 'p'),
    ];

    #[must_use]
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// The platform's attribute word for `metadata`: the mode on Unix, the
    /// attribute flags on Windows, and nothing anywhere else.
    #[must_use]
    pub fn from_metadata(metadata: &std::fs::Metadata) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Self(metadata.mode())
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            Self(metadata.file_attributes())
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = metadata;
            Self::default()
        }
    }

    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// `ls -l` style: the file type, then owner, group and other permissions,
    /// with the setuid, setgid and sticky bits in their usual places.
    #[must_use]
    pub fn mode_string(self) -> String {
        let bits = self.0;
        let kind = Self::UNIX_TYPES
            .iter()
            .find(|(mask, _)| bits & 0o170_000 == *mask)
            .map_or('?', |(_, letter)| *letter);
        let mut out = String::with_capacity(10);
        out.push(kind);
        for (shift, special, special_letter) in
            [(6, 0o4000, 's'), (3, 0o2000, 's'), (0, 0o1000, 't')]
        {
            let triplet = (bits >> shift) & 0o7;
            out.push(if triplet & 0o4 != 0 { 'r' } else { '-' });
            out.push(if triplet & 0o2 != 0 { 'w' } else { '-' });
            out.push(execute_letter(
                triplet & 0o1 != 0,
                bits & special != 0,
                special_letter,
            ));
        }
        out
    }

    /// Far's attribute letters: `R`ead-only, `H`idden, `S`ystem, `A`rchive —
    /// those set, in that order.
    #[must_use]
    pub fn flag_letters(self) -> String {
        Self::WINDOWS_LETTERS
            .iter()
            .filter(|(flag, _)| self.0 & flag != 0)
            .map(|(_, letter)| *letter)
            .collect()
    }
}

/// The third character of a permission triplet: `x` when executable, or the
/// special bit's letter — uppercase when the special bit is set without it.
fn execute_letter(executable: bool, special: bool, letter: char) -> char {
    match (executable, special) {
        (true, true) => letter,
        (false, true) => letter.to_ascii_uppercase(),
        (true, false) => 'x',
        (false, false) => '-',
    }
}

impl std::fmt::Display for Attributes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if cfg!(windows) {
            f.write_str(&self.flag_letters())
        } else {
            f.write_str(&self.mode_string())
        }
    }
}
