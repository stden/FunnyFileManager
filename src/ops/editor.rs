//! Persist the built-in editor buffer while preserving file identity and format.

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use crate::models::{EditState, Editor};

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
        ensure_writable(&target)?;
        let content = self.format.encoding.encode(&self.content())?;

        if rewrite_in_place(&target) {
            // See `rewrite_in_place`: atomicity is the lesser thing to lose.
            fs::write(&target, content)?;
            self.state = EditState::Clean;
            self.lsp.saved = self.lsp.saved.saturating_add(1);
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
        temp.write_all(&content)?;
        temp.as_file().sync_all()?;
        temp.persist(&target).map_err(|err| err.error)?;
        self.state = EditState::Clean;
        self.lsp.saved = self.lsp.saved.saturating_add(1);
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

/// Refuse a save the filesystem would otherwise let through for the wrong
/// reason.
///
/// Persisting the temp file is `rename(2)`, which replaces a directory
/// *entry*: the kernel checks write permission on the parent directory and
/// never looks at the file. A root-owned 0644 file sitting in the user's own
/// directory was therefore replaced in silence — new inode, new owner, ACLs
/// and xattrs gone — while the editor reported "Saved". The in-place branch
/// below fails on its own, because `fs::write` really opens the file, but the
/// check sits ahead of both so that one refusal has one message.
fn ensure_writable(target: &Path) -> io::Result<()> {
    match refusal(target) {
        None => Ok(()),
        Some(reason) => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("{}: {reason}", target.display()),
        )),
    }
}

/// Whether *this* process may write the file's contents.
///
/// `metadata().permissions().readonly()` reads the owner-write bit alone and
/// says nothing about who is asking, so a root-owned 0644 file looks writable
/// to every user on the machine. Only the kernel knows the real answer, and
/// only the kernel accounts for group membership, POSIX ACLs, a read-only
/// mount and root's `CAP_DAC_OVERRIDE` — so root is not blocked, which is
/// correct. `AT_EACCESS` asks with the *effective* ids, which are the ones the
/// save will use; plain `access(2)` would ask with the real ones.
///
/// A path with nothing behind it yet (Shift+F4) answers `ENOENT`, and an
/// unexpected errno — an interior NUL included — means the question could not
/// be put at all: both fall open, leaving the write that follows to report the
/// real failure rather than this guard inventing one.
#[cfg(unix)]
fn refusal(path: &Path) -> Option<&'static str> {
    let Err(error) = crate::platform::check_write_access(path) else {
        return None;
    };
    // "Permission denied" would send the user to `ls -l` and a chmod that
    // cannot help, so a read-only filesystem says which it is.
    match error.raw_os_error() {
        Some(libc::EROFS) => Some("read-only file system"),
        Some(libc::EACCES | libc::EPERM) => Some("permission denied"),
        // Anything else — the file is not there yet (Shift+F4), or the
        // question could not be put at all — falls open, so the write reports
        // what really happens instead of this guard inventing an answer.
        _ => None,
    }
}

/// Windows has one writability bit per file and `readonly()` reads exactly
/// it. A file that is not there yet is the directory's business, as on Unix.
#[cfg(not(unix))]
fn refusal(path: &Path) -> Option<&'static str> {
    fs::metadata(path)
        .is_ok_and(|metadata| metadata.permissions().readonly())
        .then_some("the file is marked read-only")
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
    let content =
        crate::fileload::read_for_text_ui(&editor.path).and_then(|loaded| match loaded {
            crate::fileload::Loaded::Bytes(bytes) => encoding.decode(&bytes),
            _ => Err(io::Error::other("Cannot reload this file as text")),
        })?;
    let mut reloaded = Editor::from_content(editor.title.clone(), editor.path.clone(), &content);
    reloaded.cursor_line = editor
        .cursor_line
        .min(reloaded.lines.len().saturating_sub(1));
    reloaded.cursor_col = editor
        .cursor_col
        .min(reloaded.lines[reloaded.cursor_line].chars().count());
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
