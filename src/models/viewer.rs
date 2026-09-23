//! Purpose: Built-in file viewer state (F3).
//!
//! Split out of `models.rs` when the viewer grew a file behind it — a path, a
//! size and an encoding for Far's status line — and the parent file was
//! already against the 1000-line cap `tests/file_line_limit_test.rs`
//! enforces. Wired the way `models/editor.rs` is.

use std::path::PathBuf;

use super::{FileSize, Viewport};
use crate::encoding::TextEncoding;
use crate::scroll::{ScrollLeft, ScrollTop};

/// What an empty file shows, so a blank screen is not mistaken for a failure.
const EMPTY_FILE: &str = "<empty file>";

/// The file behind a viewer, when there is one — a report has none.
///
/// Recorded when the file is opened, like the editor's `FileFormat`: the
/// status line redraws every frame, and re-reading the file to learn its size
/// each time would be paying again for a fact we had.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewedFile {
    pub path: PathBuf,
    pub size: FileSize,
    pub encoding: TextEncoding,
}

/// Where a viewer's lines came from: a file on disk, or a report the program
/// composed (`:mcp`, the Windows diagnostics).
///
/// An enum rather than a title plus an optional file, so a report is
/// structurally unable to carry a size or an encoding, and the status line
/// has one exhaustive match to draw from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewSource {
    /// A report, named by its title.
    Report(String),
    File(ViewedFile),
}

/// Built-in file viewer, opened with F3
#[derive(Debug, Clone)]
pub struct Viewer {
    pub source: ViewSource,
    pub lines: Vec<String>,
    /// Anchored at the *top*, unlike the console transcript's offset — see
    /// [`crate::scroll`].
    pub scroll: ScrollTop,
    /// Cells scrolled off the left edge — Far's Left and Right.
    pub col: ScrollLeft,
}

impl Viewer {
    /// Rows the viewer spends on chrome: its status line, and nothing else.
    pub const CHROME_ROWS: u16 = 1;

    /// A viewer over lines that came from nowhere on disk: a status report.
    #[must_use]
    pub fn report(title: impl Into<String>, lines: Vec<String>) -> Self {
        Self::new(ViewSource::Report(title.into()), lines)
    }

    /// A viewer over a file's bytes, decoded the way F4 decodes them.
    ///
    /// [`TextEncoding::detect`] is the editor's: a byte-order mark names UTF-8
    /// or UTF-16, valid UTF-8 is UTF-8, and anything else is the single-byte
    /// code page the editor falls back to. So F3 and F4 never show one file
    /// two ways, and the status line names a code page that was really
    /// applied. Lossy, because the viewer never writes back: a byte the code
    /// page cannot map shows as a replacement mark instead of failing the view.
    #[must_use]
    pub fn file(path: PathBuf, bytes: &[u8]) -> Self {
        let encoding = TextEncoding::detect(bytes);
        let text = encoding.decode_lossy(bytes);
        let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
        if lines.is_empty() {
            lines.push(EMPTY_FILE.to_owned());
        }
        let size = FileSize::from_bytes(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
        Self::new(
            ViewSource::File(ViewedFile {
                path,
                size,
                encoding,
            }),
            lines,
        )
    }

    const fn new(source: ViewSource, lines: Vec<String>) -> Self {
        Self {
            source,
            lines,
            scroll: ScrollTop::TOP,
            col: ScrollLeft::LEFT,
        }
    }

    /// The body rows a terminal `viewport` tall gives the file — the distance
    /// PgUp/PgDn move — never zero, so a one-row terminal still scrolls.
    #[must_use]
    pub fn page_for(viewport: Viewport) -> usize {
        usize::from(viewport.rows().saturating_sub(Self::CHROME_ROWS)).max(1)
    }

    /// Furthest the view may scroll on a `page`-row body: the last line stops
    /// on the bottom row, as Far's does, instead of climbing to the top and
    /// leaving the screen empty below it.
    fn max_scroll(&self, page: usize) -> usize {
        self.lines.len().saturating_sub(page)
    }

    /// Keep the view within a `page`-row body — what a terminal that grew
    /// taller after End needs, so no blank rows open up under the last line.
    pub fn clamp(&mut self, page: usize) {
        self.scroll_down(0, page);
    }

    pub fn scroll_down(&mut self, lines: usize, page: usize) {
        self.scroll = self.scroll.down(lines, self.max_scroll(page));
    }

    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll = self.scroll.up(lines);
    }

    /// Far's Home: the first screen, back at the left edge.
    pub fn scroll_to_top(&mut self) {
        self.scroll = ScrollTop::TOP;
        self.col = ScrollLeft::LEFT;
    }

    /// Far's End: the last screen, back at the left edge.
    pub fn scroll_to_bottom(&mut self, page: usize) {
        let last = self.max_scroll(page);
        self.scroll = ScrollTop::TOP.down(last, last);
        self.col = ScrollLeft::LEFT;
    }

    pub fn scroll_left(&mut self, cells: usize) {
        self.col = self.col.left(cells);
    }

    pub fn scroll_right(&mut self, cells: usize) {
        self.col = self.col.right(cells);
    }

    /// The name on the status line's left: a report's title, or a file's name.
    #[must_use]
    pub fn title(&self) -> &str {
        match &self.source {
            ViewSource::Report(title) => title,
            ViewSource::File(file) => file
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(""),
        }
    }

    /// The file behind this viewer, if it is one.
    #[must_use]
    pub const fn viewed_file(&self) -> Option<&ViewedFile> {
        match &self.source {
            ViewSource::File(file) => Some(file),
            ViewSource::Report(_) => None,
        }
    }
}

impl Default for Viewer {
    fn default() -> Self {
        Self::report(String::new(), Vec::new())
    }
}

#[cfg(test)]
mod tests;
