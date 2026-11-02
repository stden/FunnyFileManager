//! Purpose: How the edited file is written on disk — its line endings, final
//! newline and indentation — and how wide the editor draws a TAB.

/// End-of-line style of the file being edited.
///
/// Recorded at open time so a CRLF file (a `.bat`, or any file committed with
/// CRLF) is not silently rewritten with LF endings the first time F2 is pressed.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum LineEnding {
    #[default]
    Lf,
    Crlf,
}

impl LineEnding {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::Crlf => "\r\n",
        }
    }

    /// CRLF only when *every* line break in `content` is one.
    ///
    /// A file that mixes the two is split on LF alone, and the CR of each
    /// CRLF line stays part of that line's text, as Vim keeps it. Choosing
    /// the majority instead wrote that one ending after every line, so the
    /// minority lines were rewritten although nobody had touched them.
    fn detect(content: &str) -> Self {
        let bytes = content.as_bytes();
        let mut iter = memchr::memchr_iter(b'\n', bytes);
        let Some(first) = iter.next() else {
            return Self::Lf;
        };
        if first > 0
            && bytes.get(first - 1) == Some(&b'\r')
            && iter.all(|at| at > 0 && bytes.get(at - 1) == Some(&b'\r'))
        {
            Self::Crlf
        } else {
            Self::Lf
        }
    }
}

/// How a file is written on disk, so saving can put it back the way it came.
///
/// The two facts were separate fields — an enum for the EOL style beside a
/// bool for the final newline — even though they are read together, written
/// together, and detected together at load. Grouping them means a future third
/// fact (a BOM, say) has an obvious home, and that "the file's shape" is one
/// thing to pass around rather than two to keep in step.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FileFormat {
    pub encoding: crate::encoding::TextEncoding,
    /// The file's own EOL style.
    pub line_ending: LineEnding,
    /// Whether the original file ended with a newline.
    pub trailing_newline: bool,
    /// What the Tab key types into it.
    pub indent: Indent,
}

impl FileFormat {
    /// The shape of `content` as it was read.
    #[must_use]
    pub fn detect(content: &str) -> Self {
        Self {
            encoding: crate::encoding::TextEncoding::default(),
            line_ending: LineEnding::detect(content),
            trailing_newline: content.ends_with('\n'),
            indent: Indent::detect(content),
        }
    }
}

/// What the Tab key types into a file: the file's own convention.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Indent {
    /// [`TabSize`] spaces, the editor's setting, for a file with no TABs.
    #[default]
    Spaces,
    /// A TAB character, for a file that already contains them: spaces there
    /// would change its format — a Makefile recipe, a TSV column, the user
    /// menu's `Label<TAB>command` separator.
    Tabs,
}

impl Indent {
    fn detect(content: &str) -> Self {
        if memchr::memchr(b'\t', content.as_bytes()).is_some() {
            Self::Tabs
        } else {
            Self::Spaces
        }
    }
}

/// Columns from one tab stop to the next: how wide the editor draws a TAB,
/// and how many spaces the Tab key types into a file that does not use TABs.
///
/// It is the Options menu's "Editor settings" value, taken when the file is
/// opened. A byte, because the editor is the largest overlay and the value is
/// validated to 1–16; never zero, because a zero-wide stop would draw a TAB
/// as nothing again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TabSize(u8);

impl TabSize {
    /// The Options menu's default.
    pub const DEFAULT: Self = Self(4);

    /// A tab size of `columns`, kept to at least one column.
    #[must_use]
    pub fn new(columns: usize) -> Self {
        Self(u8::try_from(columns).unwrap_or(u8::MAX).max(1))
    }

    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "u8 to usize widens losslessly, and `usize::from` is not const"
    )]
    pub const fn columns(self) -> usize {
        self.0 as usize
    }
}

impl Default for TabSize {
    fn default() -> Self {
        Self::DEFAULT
    }
}
