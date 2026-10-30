//! Purpose: Post-run cleanup for `Shift+F1` archive creation: removing
//! incomplete archives on failure and deleting archived sources when
//! `delete_after` is ticked without destroying the newly created archive.

use std::fs;
use std::io;
use std::path::Path;

use crate::models::App;
use crate::ops::wipe::{clear_readonly, remove_symlink};

/// Remove what a failed run left at `archive`, which this run created: tar
/// writes every member it can read before it reports the one it could not,
/// and the half that is left looks like a finished archive.
pub(super) fn discard_partial(app: &mut App, archive: &Path) {
    let Err(error) = fs::remove_file(archive) else {
        return;
    };
    if error.kind() != io::ErrorKind::NotFound {
        let shown = archive.display();
        let text = format!("{shown}: the incomplete archive could not be removed: {error}");
        app.console.record_error(&text);
    }
}

/// Remove every source except `archive` itself; the count of those that would
/// not go.
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
        clear_readonly(path);
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
    clear_readonly(dir);
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
    fs::remove_dir(dir)
}

fn remove_file_or_link(path: &Path, file_type: fs::FileType) -> io::Result<()> {
    if !file_type.is_symlink() {
        clear_readonly(path);
    }
    remove_symlink(path, file_type)
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
