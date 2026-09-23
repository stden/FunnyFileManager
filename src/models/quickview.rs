//! Purpose: What the Quick view panel (`Ctrl+Q`) is showing.
//!
//! The preview is loaded by the operation that reacts to the other panel's
//! cursor and kept here, because the renderer may not read the disk: a panel
//! that re-read the file on every keystroke would be paying for a fact it
//! already had.

use std::path::{Path, PathBuf};

use super::{FileSize, Viewer, Viewport};
use crate::encoding::TextEncoding;
use crate::scroll::{ScrollLeft, ScrollTop};
use crate::stats::DirectoryStats;

/// The entry under the other panel's cursor and its preview.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuickView {
    /// The path the preview was loaded for, or `None` while the other cursor
    /// is on `..` or the panel has not been asked yet. Compared on every
    /// event, so the disk is read only when the cursor has actually moved.
    pub shown: Option<PathBuf>,
    pub preview: Preview,
    /// Anchored at the top, like the viewer's.
    pub scroll: ScrollTop,
    /// Cells scrolled off the left edge — Far's Left and Right.
    pub col: ScrollLeft,
}

/// What the panel has to show for the path under the other cursor.
///
/// An enum rather than lines plus flags, so a folder's totals cannot carry an
/// encoding and a refusal cannot be scrolled: the renderer has one exhaustive
/// match to draw from and the keys one question to ask.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Preview {
    /// Nothing under the other cursor: `..`, or an empty listing.
    #[default]
    Nothing,
    /// A text file, decoded the way the viewer decodes it.
    Text(TextPreview),
    /// A folder's totals when no description is shown.
    Folder(Box<DirectoryStats>),
    /// A file the preview declined, and why.
    Refused(Refusal),
}

/// A decoded text file: the lines, and what the viewer's status line would
/// say about them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextPreview {
    /// The text source, which can be a description inside the selected folder.
    pub path: PathBuf,
    pub lines: Vec<String>,
    pub encoding: TextEncoding,
    pub size: FileSize,
}

impl TextPreview {
    /// Decode `bytes` exactly as F3 would — in the encoding the editor detects,
    /// lossily, and marking an empty file — so the preview and the viewer
    /// never disagree about a file.
    #[must_use]
    pub fn decoded(path: &Path, bytes: &[u8]) -> Self {
        let viewer = Viewer::file(path.to_path_buf(), bytes);
        let (encoding, size) = viewer
            .viewed_file()
            .map_or((TextEncoding::Utf8, FileSize::from_bytes(0)), |file| {
                (file.encoding, file.size)
            });
        Self {
            path: path.to_path_buf(),
            lines: viewer.lines,
            encoding,
            size,
        }
    }
}

/// Why a file under the cursor has no preview.
///
/// The same three refusals F3 reports in the status bar, plus the read error
/// F3 turns into a failed action; the preview has no action to fail, so it
/// says so in the panel instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// A NUL byte with no UTF-16 byte-order mark to explain it: the
    /// line-oriented preview would render garbage.
    Binary,
    /// Over the viewer's size limit; carries the size the metadata reported.
    TooLarge(FileSize),
    /// A FIFO, socket or device node — opening one could block the loop.
    NotRegular,
    /// The read itself failed: gone, denied, unreadable.
    Unreadable(String),
}

impl Refusal {
    /// The one line the panel shows in place of the file.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Binary => "Binary file: no text to show".to_owned(),
            Self::TooLarge(size) => {
                format!("Too large to preview: {:.1} MB", size.megabytes())
            }
            Self::NotRegular => "Not a regular file".to_owned(),
            Self::Unreadable(error) => format!("Cannot read: {error}"),
        }
    }
}

impl QuickView {
    /// Rows the preview cannot draw text on: the three around the panels
    /// (menu bar, command line, key bar), the frame's two, the rule and the
    /// info row above the bottom border. The same geometry
    /// `crate::ui::panel_page_size` derives for a file panel, less the column
    /// header a preview does not draw; the render test pins the two together.
    pub const CHROME_ROWS: u16 = 7;

    /// Show `preview` for `path`, from the top-left corner: a new file starts
    /// where the reader expects it, not where the last one was left.
    pub fn show(&mut self, path: Option<PathBuf>, preview: Preview) {
        self.shown = path;
        self.preview = preview;
        self.scroll = ScrollTop::TOP;
        self.col = ScrollLeft::LEFT;
    }

    /// The body rows a terminal `viewport` tall gives the preview — the
    /// distance PgUp/PgDn move — never zero, so a short terminal still pages.
    #[must_use]
    pub fn page_for(viewport: Viewport) -> usize {
        usize::from(viewport.rows().saturating_sub(Self::CHROME_ROWS)).max(1)
    }

    /// How many lines there are to scroll through: a text file's, and nothing
    /// for a folder's totals or a refusal, which fit their screen.
    #[must_use]
    pub fn line_count(&self) -> usize {
        match &self.preview {
            Preview::Text(text) => text.lines.len(),
            Preview::Nothing | Preview::Folder(_) | Preview::Refused(_) => 0,
        }
    }

    /// Furthest the view may scroll on a `page`-row body: the last line stops
    /// on the bottom row, as Far's does.
    fn max_scroll(&self, page: usize) -> usize {
        self.line_count().saturating_sub(page)
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
}

#[cfg(test)]
mod tests;
