//! Built-in text editor state (F4).

use std::path::PathBuf;

mod disk;
pub mod lsp;
mod selection;
pub use disk::{AfterSave, Fingerprint, OverwritePrompt};
pub use lsp::{EditorLspState, LspFeature, ServerState};
pub use selection::{EditorMotion, EditorSelection, SelectionMode};
#[cfg(test)]
mod tests;

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
    const fn as_str(self) -> &'static str {
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
        let breaks = content.matches('\n').count();
        if breaks > 0 && content.matches("\r\n").count() == breaks {
            Self::Crlf
        } else {
            Self::Lf
        }
    }
}

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
        if content.contains('\t') {
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
    pub const fn columns(self) -> usize {
        self.0 as usize
    }
}

impl Default for TabSize {
    fn default() -> Self {
        Self::DEFAULT
    }
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

/// Where the buffer stands relative to the file on disk.
///
/// This replaced `modified: bool` beside `confirm_exit: bool`, which made
/// "asking whether to save, with nothing to save" a representable state. The
/// exit dialog exists *because* there are unsaved changes, so it is a stage of
/// being modified rather than an independent flag.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EditState {
    /// Saved, or never touched.
    #[default]
    Clean,
    /// Edited since the last save.
    Modified,
    /// Esc was pressed with unsaved changes; awaiting save/discard/cancel.
    ///
    /// Carries which button the cursor is on. An `Option<usize>` index would
    /// have states naming no button at all, and every caller would then have to
    /// defend against them; three named answers cannot be out of range.
    ConfirmingExit(ExitChoice),
    /// A save found the file changed on disk since it was read; awaiting
    /// Overwrite or Cancel.
    ConfirmingOverwrite(OverwritePrompt),
}

/// The button focused in the editor's unsaved-changes dialog.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExitChoice {
    /// Save and close — the default, which is what Enter starts on.
    #[default]
    Save,
    /// Close without saving.
    Discard,
    /// Stay in the editor.
    Cancel,
}

impl ExitChoice {
    /// The next button rightwards, wrapping — which is what Far's Right and
    /// Tab both do in a dialog.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Save => Self::Discard,
            Self::Discard => Self::Cancel,
            Self::Cancel => Self::Save,
        }
    }

    #[must_use]
    pub const fn previous(self) -> Self {
        match self {
            Self::Save => Self::Cancel,
            Self::Discard => Self::Save,
            Self::Cancel => Self::Discard,
        }
    }
}

impl EditState {
    /// Whether the buffer differs from the file — true in both edited stages.
    #[must_use]
    pub const fn is_modified(self) -> bool {
        matches!(
            self,
            Self::Modified | Self::ConfirmingExit(_) | Self::ConfirmingOverwrite(_)
        )
    }

    #[must_use]
    pub const fn is_confirming_exit(self) -> bool {
        matches!(self, Self::ConfirmingExit(_))
    }

    /// Which button the exit dialog has focus on, if it is open.
    #[must_use]
    pub const fn exit_choice(self) -> Option<ExitChoice> {
        match self {
            Self::ConfirmingExit(choice) => Some(choice),
            _ => None,
        }
    }

    /// The "changed on disk" question, if it is open.
    #[must_use]
    pub const fn overwrite_prompt(self) -> Option<OverwritePrompt> {
        match self {
            Self::ConfirmingOverwrite(prompt) => Some(prompt),
            _ => None,
        }
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

    pub fn insert_char(&mut self, ch: char) {
        self.delete_selection();
        self.ensure_line();
        let at = self.byte_col(self.cursor_col);
        self.lines[self.cursor_line].insert(at, ch);
        self.cursor_col += 1;
        self.mark_modified();
    }

    pub fn insert_newline(&mut self) {
        self.delete_selection();
        self.ensure_line();
        let at = self.byte_col(self.cursor_col);
        let rest = self.lines[self.cursor_line].split_off(at);
        self.lines.insert(self.cursor_line + 1, rest);
        self.cursor_line += 1;
        self.cursor_col = 0;
        self.mark_modified();
    }

    /// Delete the char before the cursor, joining with the previous line at column 0.
    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor_col > 0 {
            let at = self.byte_col(self.cursor_col - 1);
            self.lines[self.cursor_line].remove(at);
            self.cursor_col -= 1;
            self.mark_modified();
        } else if self.cursor_line > 0 {
            let current = self.lines.remove(self.cursor_line);
            self.cursor_line -= 1;
            self.cursor_col = self.line_char_len();
            self.lines[self.cursor_line].push_str(&current);
            self.mark_modified();
        }
    }

    /// Delete the char under the cursor, joining with the next line at end of line.
    pub fn delete_forward(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor_col < self.line_char_len() {
            let at = self.byte_col(self.cursor_col);
            self.lines[self.cursor_line].remove(at);
            self.mark_modified();
        } else if self.cursor_line + 1 < self.lines.len() {
            let next = self.lines.remove(self.cursor_line + 1);
            self.lines[self.cursor_line].push_str(&next);
            self.mark_modified();
        }
    }

    pub fn move_left(&mut self) {
        self.selection = EditorSelection::None;
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.cursor_col = self.line_char_len();
        }
    }

    pub fn move_right(&mut self) {
        self.selection = EditorSelection::None;
        if self.cursor_col < self.line_char_len() {
            self.cursor_col += 1;
        } else if self.cursor_line + 1 < self.lines.len() {
            self.cursor_line += 1;
            self.cursor_col = 0;
        }
    }

    pub const fn move_home(&mut self) {
        self.selection = EditorSelection::None;
        self.cursor_col = 0;
    }

    pub fn move_end(&mut self) {
        self.selection = EditorSelection::None;
        self.cursor_col = self.line_char_len();
    }

    pub fn move_up(&mut self, count: usize) {
        self.selection = EditorSelection::None;
        self.cursor_line = self.cursor_line.saturating_sub(count);
        self.clamp_col();
    }

    pub fn move_down(&mut self, count: usize) {
        self.selection = EditorSelection::None;
        self.cursor_line = self
            .cursor_line
            .saturating_add(count)
            .min(self.lines.len().saturating_sub(1));
        self.clamp_col();
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
