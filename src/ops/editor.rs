//! Persist the built-in editor buffer while preserving file identity and format.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::models::{EditState, Editor, Fingerprint};

mod clipboard;
pub use clipboard::{copy_editor_selection, paste_into_editor};

impl Editor {
    /// Write the buffer back to the file behind `self.path`, clearing the
    /// modified flag.
    ///
    /// Writes a sibling temp file and renames it over the target. `fs::write`
    /// truncates the original before the first new byte lands, so a full disk or
    /// a crash mid-write would leave nothing but an empty file behind.
    pub fn save(&mut self) -> io::Result<()> {
        let target = resolve_save_target(&self.path);
        crate::file_write::ensure_writable(&target)?;
        let content = self.format.encoding.encode(&self.content())?;
        write_file(&target, &content)?;
        self.mark_saved(&content);
        Ok(())
    }

    /// Whether another program has written the file since the editor last
    /// read or saved it — the question a save has to ask before it replaces
    /// that write.
    ///
    /// Read through the same bounded loader as F4, so a FIFO or a huge file
    /// put in its place cannot hang the save; either one, like a file that is
    /// gone, has changed. A file that cannot be read cannot be compared, and
    /// the write that follows reports its own error.
    #[must_use]
    pub fn changed_on_disk(&self) -> bool {
        let Some(read) = self.disk else {
            return false;
        };
        match crate::fileload::read_for_text_ui(&self.path) {
            Ok(crate::fileload::Loaded::Bytes(bytes)) => Fingerprint::of(&bytes) != read,
            Ok(_) => true,
            Err(err) => err.kind() == io::ErrorKind::NotFound,
        }
    }

    fn mark_saved(&mut self, written: &[u8]) {
        self.state = EditState::Clean;
        self.disk = Some(Fingerprint::of(written));
        self.lsp.saved = self.lsp.saved.saturating_add(1);
    }
}

/// Put `content` in `target`: atomically, through a temp file renamed over
/// it, unless replacing the file would lose something (see
/// [`rewrite_in_place`]) or its directory cannot hold a temp file.
fn write_file(target: &Path, content: &[u8]) -> io::Result<()> {
    if rewrite_in_place(target) {
        // See `rewrite_in_place`: atomicity is the lesser thing to lose.
        return fs::write(target, content);
    }
    match crate::file_write::temp_beside(target) {
        Ok(temp) => crate::file_write::persist_over(temp, target, content),
        // A directory the user may not add to can still hold a file they may
        // write — `ensure_writable` has just said so. Write through it, as vim
        // and nano do, rather than refuse a save the user is entitled to.
        Err(err) if err.kind() == io::ErrorKind::PermissionDenied && target.exists() => {
            fs::write(target, content)
        }
        Err(err) => Err(err),
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

/// Whether the save writes through the file rather than replacing it.
///
/// Replacing means `rename(2)`, which makes a new inode: the file's other
/// names stop following it, and it belongs to whoever saved it. A file this
/// user does not own is therefore written through — editing a colleague's
/// 0664 file in a shared directory must not quietly transfer it to the editor.
/// Losing atomicity is the lesser evil in both cases, and it is the trade-off
/// vim makes by default (`backupcopy=auto`).
fn rewrite_in_place(path: &Path) -> bool {
    has_other_names(path) || !owned_by_us(path)
}

/// Whether this user owns the file. Unknown counts as owned: the atomic path
/// is the better one, and a file that is not there yet has no owner to keep.
#[cfg(unix)]
fn owned_by_us(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;

    let us = crate::platform::effective_uid();
    fs::metadata(path).map_or(true, |info| info.uid() == us)
}

/// Windows has no cheap owner comparison here, and its ACLs survive a rename
/// differently; the atomic path stays.
#[cfg(not(unix))]
const fn owned_by_us(_path: &Path) -> bool {
    true
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

#[cfg(test)]
mod tests;

pub fn change_editor_encoding(
    app: &mut crate::models::App,
    encoding: crate::encoding::TextEncoding,
) {
    let Some(editor) = app.get_editor_mut() else {
        return;
    };
    let previous = editor.format.encoding;
    editor.format.encoding = encoding;
    if encoding != previous {
        editor.state = EditState::Modified;
    }
    editor.notice = Some(format!(
        "Encoding: {} (F8 changes encoding)",
        encoding.label()
    ));
}

fn reload_editor(editor: &mut Editor, encoding: crate::encoding::TextEncoding) -> io::Result<()> {
    let crate::fileload::Loaded::Bytes(bytes) = crate::fileload::read_for_text_ui(&editor.path)?
    else {
        return Err(io::Error::other("Cannot reload this file as text"));
    };
    let content = encoding.decode(&bytes)?;
    let mut reloaded = Editor::from_content(editor.title.clone(), editor.path.clone(), &content);
    reloaded.disk = Some(Fingerprint::of(&bytes));
    reloaded.tab_size = editor.tab_size;
    reloaded.cursor_line = editor
        .cursor_line
        .min(reloaded.lines.len().saturating_sub(1));
    let line_chars = reloaded
        .lines
        .get(reloaded.cursor_line)
        .map_or(0, |line| line.chars().count());
    reloaded.cursor_col = editor.cursor_col.min(line_chars);
    *editor = reloaded;
    Ok(())
}
pub fn reload_editor_encoding(
    app: &mut crate::models::App,
    encoding: crate::encoding::TextEncoding,
) {
    let Some(editor) = app.get_editor_mut() else {
        return;
    };
    if editor.state.is_modified() {
        editor.notice = Some("Save or discard changes before reloading the file".to_owned());
        return;
    }
    match reload_editor(editor, encoding) {
        Ok(()) => {
            editor.format.encoding = encoding;
            editor.notice = Some(format!("Reloaded as {}", encoding.label()));
        }
        Err(error) => editor.notice = Some(error.to_string()),
    }
}
