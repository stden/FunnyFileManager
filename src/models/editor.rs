//! Built-in text editor state (F4).

use std::path::PathBuf;

mod disk;
mod edit_state;
mod editing;
mod format;
pub mod lsp;
mod movement;
mod pretty;
mod selection;
pub use disk::{AfterSave, Fingerprint, OverwritePrompt};
pub use edit_state::{EditState, ExitChoice};
pub use format::{FileFormat, Indent, LineEnding, TabSize};
pub use lsp::{EditorLspState, LspFeature, ServerState};
pub use selection::{EditorMotion, EditorSelection, SelectionMode};
#[cfg(test)]
mod tests;

/// Built-in text editor, opened with F4 (Far Manager style).
///
/// The cursor column is measured in *chars*, not bytes, so editing
/// multi-byte (Unicode) lines stays panic-free.
#[derive(Default, Clone)]
pub struct Editor {
    pub lsp: EditorLspState,
    pub title: String,
    pub path: PathBuf,
    pub lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    /// Whether the buffer differs from the file, and whether the exit dialog
    /// is up. One field, because the second question only exists inside the
    /// first — see [`EditState`].
    pub state: EditState,
    /// How the file was written on disk, restored verbatim on save.
    pub format: FileFormat,
    /// Where the tab stops are, for drawing TABs and for the Tab key.
    pub tab_size: TabSize,
    /// What the disk held when the file was last read or saved, so a save
    /// can notice another program's write in between; `None` for a buffer
    /// that was not read from a file, whose saves do not ask.
    pub disk: Option<Fingerprint>,
    /// F7's search: the dialog while it is open, and what Shift+F7 repeats.
    pub search: crate::search::SearchState,
    /// Stream selection anchored independently from the moving caret.
    pub selection: EditorSelection,
    /// Transient feedback for editor operations such as clipboard access.
    pub notice: Option<String>,
    /// Selected encoding while the F8 dialog is open.
    pub encoding_choice: Option<crate::encoding::TextEncoding>,
    /// Lexical cache; it checks source equality because callers can edit `lines` directly.
    pub highlights: std::cell::RefCell<crate::highlight::cache::HighlightCache>,
}

/// A caret's position within a line, counted in **characters**.
///
/// The third unit in a triangle the rest of the program already names: bytes
/// are [`Caret`](crate::lineedit::Caret) and screen columns are
/// [`Cells`](crate::ui::Cells). Only the *boundary* is typed — inside the
/// editor the column is plain arithmetic on a `usize`, and wrapping every
/// `+= 1` would make that code harder to read for no gain, since nothing else
/// in there measures anything else. What needed a name is the value that
/// leaves: `ui::EditorLine::cells_before` converts it to a column, and handing that
/// function a byte offset or a cell count would silently misplace the caret.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct CharColumn(usize);

impl CharColumn {
    #[must_use]
    pub const fn new(chars: usize) -> Self {
        Self(chars)
    }

    #[must_use]
    pub const fn chars(self) -> usize {
        self.0
    }
}

impl Editor {
    /// The caret's column, in characters — the unit the editor moves it in.
    #[must_use]
    pub const fn cursor_column(&self) -> CharColumn {
        CharColumn::new(self.cursor_col)
    }

    /// Build an editor from raw file content.
    ///
    /// Splits on the file's own [`LineEnding`] and nothing else, so that
    /// [`Self::content`] gives back exactly `content` until something is
    /// edited. `str::lines` could not: it also strips the CR of a CRLF line in
    /// an LF file, which the save then never wrote back.
    #[must_use]
    pub fn from_content(title: String, path: PathBuf, content: &str) -> Self {
        let format = FileFormat::detect(content);
        let eol = format.line_ending.as_str();
        let body = if format.trailing_newline {
            content.strip_suffix(eol).unwrap_or(content)
        } else {
            content
        };
        Self {
            title,
            path,
            lines: body.split(eol).map(str::to_owned).collect(),
            format,
            ..Self::default()
        }
    }

    /// Where the caret is, in the units [`crate::search`] works in.
    #[must_use]
    pub const fn caret(&self) -> crate::search::Caret {
        crate::search::Caret::new(self.cursor_line, self.cursor_col)
    }

    /// Move the caret to the next match, reporting whether there was one.
    ///
    /// Returns a `bool` rather than moving silently: a search that found
    /// nothing and a search that landed back where it started look identical on
    /// screen, and only the caller can say which happened.
    pub fn find(&mut self, search: &crate::search::Search) -> bool {
        let Some(at) = search.find(&self.lines, self.caret()) else {
            return false;
        };
        self.cursor_line = at.line.get();
        self.cursor_col = at.column.chars();
        self.selection = EditorSelection::None;
        true
    }

    fn line(&self) -> &str {
        self.lines.get(self.cursor_line).map_or("", String::as_str)
    }

    /// The caret's line, for editing in place. Always there once
    /// `ensure_line` has run.
    fn line_mut(&mut self) -> Option<&mut String> {
        self.lines.get_mut(self.cursor_line)
    }

    fn line_char_len(&self) -> usize {
        self.line().chars().count()
    }

    /// Byte index of the given char column in the current line.
    fn byte_col(&self, col: usize) -> usize {
        let line = self.line();
        line.char_indices().nth(col).map_or(line.len(), |(i, _)| i)
    }

    fn clamp_col(&mut self) {
        self.cursor_col = self.cursor_col.min(self.line_char_len());
    }

    /// Full file content as it would be written to disk.
    #[must_use]
    pub fn content(&self) -> String {
        let eol = self.format.line_ending.as_str();
        let mut content = self.lines.join(eol);
        if self.format.trailing_newline {
            content.push_str(eol);
        }
        content
    }
}
