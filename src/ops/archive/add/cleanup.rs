//! Purpose: Post-run cleanup for `Shift+F1` archive creation: removing
//! incomplete archives on failure and deleting archived sources when
//! `delete_after` is ticked without destroying the newly created archive.

use std::fs;
use std::io;
use std::path::Path;

use crate::models::App;

/// Remove what a failed run left at `archive`, which this run created: tar
/// writes every member it can read before it reports the one it could not,
/// and the half that is left looks like a finished archive. Never called for
/// an archive that was there before — a failed append may have changed it,
/// but removing it would lose what it held. A removal that fails is said in
/// the transcript; the run's own failure stays the status.
pub(super) fn discard_partial(app: &mut App, archive: &Path) {
    let Err(error) = fs::remove_file(archive) else {
        return;
    };
    // Nothing written at all — the tool never started — is nothing to remove.
    if error.kind() != io::ErrorKind::NotFound {
        let shown = archive.display();
        let text = format!("{shown}: the incomplete archive could not be removed: {error}");
        app.console.record_error(&text);
    }
}

/// Remove every source except `archive` itself; the count of those that would
/// not go.
///
/// Only reached after the archive is written, so a source that stays is a
/// leftover, never a loss. When `archive` lives inside a source folder, the
/// other entries in that folder are removed while `archive` is kept.
pub(super) fn delete_sources(cwd: &Path, sources: &[String], archive: &Path) -> usize {
    sources
        .iter()
        .map(|name| cwd.join(name))
        .filter(|path| !same_path(path, archive))
        .filter(|path| remove_source(path, archive).is_err())
        .count()
}

fn remove_source(path: &Path, archive: &Path) -> io::Result<()> {
    if same_path(path, archive) {
        return Ok(());
    }
    let file_type = fs::symlink_metadata(path)?.file_type();
    if !file_type.is_dir() {
        return remove_file_or_link(path, file_type);
    }
    if contains_archive(path, archive) {
        let mut first_error = None;
        for entry in fs::read_dir(path)? {
            let result = entry.and_then(|e| remove_source(&e.path(), archive));
            if let Err(err) = result {
                first_error.get_or_insert(err);
            }
        }
        return first_error.map_or(Ok(()), Err);
    }
    remove_dir_tree(path)
}

fn remove_dir_tree(dir: &Path) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            remove_dir_tree(&path)?;
        } else {
            remove_file_or_link(&path, file_type)?;
        }
    }
    clear_readonly(dir);
    fs::remove_dir(dir)
}

fn remove_file_or_link(path: &Path, file_type: fs::FileType) -> io::Result<()> {
    clear_readonly(path);
    #[cfg(windows)]
    if std::os::windows::fs::FileTypeExt::is_symlink_dir(&file_type) {
        return fs::remove_dir(path).or_else(|_| fs::remove_file(path));
    }
    let _ = file_type;
    fs::remove_file(path)
}

#[expect(
    clippy::permissions_set_readonly_false,
    reason = "clears the read-only attribute before deleting an archived source entry"
)]
fn clear_readonly(path: &Path) {
    if let Ok(meta) = fs::symlink_metadata(path) {
        let mut perms = meta.permissions();
        if perms.readonly() {
            perms.set_readonly(false);
            drop(fs::set_permissions(path, perms));
        }
    }
}

fn same_path(a: &Path, b: &Path) -> bool {
    a == b
        || matches!(
            (a.canonicalize(), b.canonicalize()),
            (Ok(ra), Ok(rb)) if ra == rb
        )
}

fn contains_archive(dir: &Path, archive: &Path) -> bool {
    archive.starts_with(dir)
        || matches!(
            (dir.canonicalize(), archive.canonicalize()),
            (Ok(rdir), Ok(rarch)) if rarch.starts_with(&rdir)
        )
}
