//! Purpose: Find nested working copies without entering Git data or directory links.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

const EXCLUDED: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    ".venv",
    "venv",
    "__pycache__",
];

#[derive(Default)]
pub(super) struct Scan {
    pub(super) repositories: Vec<PathBuf>,
    pub(super) errors: usize,
}

pub(super) fn scan(root: &Path, cancelled: &dyn Fn() -> bool) -> io::Result<Scan> {
    let mut result = Scan::default();
    let mut pending = vec![root.to_owned()];
    while let Some(path) = pending.pop() {
        crate::progress::entering(&path);
        if cancelled() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Git pull cancelled",
            ));
        }
        if is_repository(&path) {
            result.repositories.push(path.clone());
            crate::progress::examined();
        }
        if let Err(error) = descend(&path, &mut pending, &mut result.errors) {
            if path == root {
                return Err(error);
            }
            result.errors += 1;
        }
    }
    result.repositories.sort_unstable();
    Ok(result)
}

fn is_repository(path: &Path) -> bool {
    fs::symlink_metadata(path.join(".git"))
        .is_ok_and(|metadata| metadata.is_file() || metadata.is_dir())
}

fn descend(path: &Path, pending: &mut Vec<PathBuf>, errors: &mut usize) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        match entry.and_then(|entry| child_directory(&entry)) {
            Ok(Some(path)) => pending.push(path),
            Ok(None) => {}
            Err(_) => *errors += 1,
        }
    }
    Ok(())
}

fn child_directory(entry: &fs::DirEntry) -> io::Result<Option<PathBuf>> {
    let kind = entry.file_type()?;
    if !kind.is_dir() || kind.is_symlink() || EXCLUDED.iter().any(|name| entry.file_name() == *name)
    {
        return Ok(None);
    }
    Ok(Some(entry.path()))
}

#[cfg(test)]
mod tests;
