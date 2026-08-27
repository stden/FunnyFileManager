//! Built-in text editor state (F4).

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

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

    /// The dominant terminator in `content`: CRLF only when it is the majority,
    /// so a stray `\r\n` inside an LF file does not convert the whole file.
    fn detect(content: &str) -> Self {
        let crlf = content.matches("\r\n").count();
        let lf = content.matches('\n').count();
        if crlf * 2 > lf {
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
    /// The file's own EOL style.
    pub line_ending: LineEnding,
    /// Whether the original file ended with a newline.
    pub trailing_newline: bool,
}

impl FileFormat {
    /// The shape of `content` as it was read.
    #[must_use]
    pub fn detect(content: &str) -> Self {
        Self {
            line_ending: LineEnding::detect(content),
            trailing_newline: content.ends_with('\n'),
        }
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
/// leaves: `ui::cells_before` converts it to a column, and handing that
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
    ConfirmingExit,
}

impl EditState {
    /// Whether the buffer differs from the file — true in both edited stages.
    #[must_use]
    pub const fn is_modified(self) -> bool {
        matches!(self, Self::Modified | Self::ConfirmingExit)
    }

    #[must_use]
    pub const fn is_confirming_exit(self) -> bool {
        matches!(self, Self::ConfirmingExit)
    }
}

impl Editor {
    /// The caret's column, in characters — the unit the editor moves it in.
    #[must_use]
    pub const fn cursor_column(&self) -> CharColumn {
        CharColumn::new(self.cursor_col)
    }

    /// Build an editor from raw file content.
    #[must_use]
    pub fn from_content(title: String, path: PathBuf, content: &str) -> Self {
        let mut lines: Vec<String> = content.lines().map(ToString::to_string).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        Self {
            title,
            path,
            lines,
            format: FileFormat::detect(content),
            ..Self::default()
        }
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
        let at = self.byte_col(self.cursor_col);
        self.lines[self.cursor_line].insert(at, ch);
        self.cursor_col += 1;
        self.state = EditState::Modified;
    }

    pub fn insert_newline(&mut self) {
        let at = self.byte_col(self.cursor_col);
        let rest = self.lines[self.cursor_line].split_off(at);
        self.lines.insert(self.cursor_line + 1, rest);
        self.cursor_line += 1;
        self.cursor_col = 0;
        self.state = EditState::Modified;
    }

    /// Delete the char before the cursor, joining with the previous line at column 0.
    pub fn backspace(&mut self) {
        if self.cursor_col > 0 {
            let at = self.byte_col(self.cursor_col - 1);
            self.lines[self.cursor_line].remove(at);
            self.cursor_col -= 1;
            self.state = EditState::Modified;
        } else if self.cursor_line > 0 {
            let current = self.lines.remove(self.cursor_line);
            self.cursor_line -= 1;
            self.cursor_col = self.line_char_len();
            self.lines[self.cursor_line].push_str(&current);
            self.state = EditState::Modified;
        }
    }

    /// Delete the char under the cursor, joining with the next line at end of line.
    pub fn delete_forward(&mut self) {
        if self.cursor_col < self.line_char_len() {
            let at = self.byte_col(self.cursor_col);
            self.lines[self.cursor_line].remove(at);
            self.state = EditState::Modified;
        } else if self.cursor_line + 1 < self.lines.len() {
            let next = self.lines.remove(self.cursor_line + 1);
            self.lines[self.cursor_line].push_str(&next);
            self.state = EditState::Modified;
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.cursor_col = self.line_char_len();
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor_col < self.line_char_len() {
            self.cursor_col += 1;
        } else if self.cursor_line + 1 < self.lines.len() {
            self.cursor_line += 1;
            self.cursor_col = 0;
        }
    }

    pub const fn move_home(&mut self) {
        self.cursor_col = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor_col = self.line_char_len();
    }

    pub fn move_up(&mut self, count: usize) {
        self.cursor_line = self.cursor_line.saturating_sub(count);
        self.clamp_col();
    }

    pub fn move_down(&mut self, count: usize) {
        self.cursor_line = (self.cursor_line + count).min(self.lines.len() - 1);
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

    /// Write the buffer back to the file behind `self.path`, clearing the
    /// modified flag.
    ///
    /// Writes a sibling temp file and renames it over the target. `fs::write`
    /// truncates the original before the first new byte lands, so a full disk or
    /// a crash mid-write would leave nothing but an empty file behind.
    pub fn save(&mut self) -> io::Result<()> {
        let target = resolve_save_target(&self.path);
        let content = self.content();

        if has_other_names(&target) {
            // Rewritten in place: see `has_other_names`.
            fs::write(&target, content)?;
            self.state = EditState::Clean;
            return Ok(());
        }

        // The temp file must share the target's directory, or the rename would
        // cross filesystems and stop being atomic.
        let dir = target
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let mut temp = tempfile::NamedTempFile::new_in(dir)?;
        // Temp files are created 0600 and the rename carries the mode onto the
        // target, so the original's permissions have to be copied over first.
        if let Ok(metadata) = fs::metadata(&target) {
            temp.as_file().set_permissions(metadata.permissions())?;
        }
        temp.write_all(content.as_bytes())?;
        temp.as_file().sync_all()?;
        temp.persist(&target).map_err(|err| err.error)?;
        self.state = EditState::Clean;
        Ok(())
    }
}

/// The file a save must actually land on.
///
/// Persisting the temp file is `rename(2)`, which replaces the *name* it is
/// given instead of writing through it. Handed `~/.vimrc -> ~/dotfiles/vimrc`,
/// the rename would drop a fresh regular file over the link and leave the real
/// file — the one under version control — holding the pre-edit text. Resolving
/// first puts both the temp file and the rename on the file the user opened.
///
/// A path with nothing behind it yet (Shift+F4) cannot be canonicalised and is
/// used as given.
fn resolve_save_target(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Whether the file has more than one name (a hard link).
///
/// A rename detaches only the name it replaces, so an atomic save would leave
/// every other name pointing at the pre-edit inode — the same silent divergence
/// a symlink suffers, and one `canonicalize` cannot resolve because hard links
/// have no "real" name. Such files are rewritten in place instead: losing
/// atomicity is the lesser evil, and it is the trade-off vim makes by default
/// (`backupcopy=auto`).
#[cfg(unix)]
fn has_other_names(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;

    fs::metadata(path).is_ok_and(|metadata| metadata.nlink() > 1)
}

/// Hard links are not reachable through the panel on non-Unix targets, so the
/// atomic path is always taken there.
#[cfg(not(unix))]
const fn has_other_names(_path: &Path) -> bool {
    false
}
