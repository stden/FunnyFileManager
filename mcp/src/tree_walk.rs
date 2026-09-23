//! Purpose: Reads directory trees for size reporting and text rendering.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

/// Return the bytes held by a file or directory tree, counting each link as
/// the link itself instead of following its target.
///
/// Not following links prevents both observing data outside the MCP root and
/// recursing forever through a link to an ancestor.
#[must_use]
pub fn directory_size(path: &Path) -> u64 {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return 0;
    };
    if !metadata.is_dir() {
        return metadata.len();
    }

    let mut bytes = 0_u64;
    let mut pending: Vec<PathBuf> = vec![path.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            bytes += entry_size(&entry, &mut pending);
        }
    }
    bytes
}

/// Classify one entry without following links; unreadable metadata contributes no bytes.
fn entry_size(entry: &fs::DirEntry, pending: &mut Vec<PathBuf>) -> u64 {
    match entry.file_type() {
        Ok(file_type) if file_type.is_dir() && !file_type.is_symlink() => {
            pending.push(entry.path());
            0
        }
        // DirEntry metadata does not follow links and reuses enumeration data on Windows.
        Ok(_) => entry.metadata().map_or(0, |metadata| metadata.len()),
        // Keep best-effort support for filesystems that supply metadata but no type.
        Err(_) => {
            let Ok(metadata) = entry.metadata() else {
                return 0;
            };
            if metadata.is_dir() {
                pending.push(entry.path());
                0
            } else {
                metadata.len()
            }
        }
    }
}

/// Render `path` as a sorted, connector-prefixed directory tree. Symlinks are
/// rendered as leaf entries and are never traversed.
#[must_use]
pub fn render_directory_tree(path: &Path, max_depth: usize) -> String {
    let mut output = path.display().to_string();
    output.push('/');
    let mut prefix = String::new();
    append_tree(path, &mut prefix, 0, max_depth, &mut output);
    output
}

struct TreeEntry {
    entry: fs::DirEntry,
    name: OsString,
    is_dir: bool,
}

fn sorted_entries(path: &Path) -> Vec<TreeEntry> {
    let mut entries: Vec<_> = match fs::read_dir(path) {
        Ok(read_dir) => read_dir
            .flatten()
            .map(|entry| TreeEntry {
                name: entry.file_name(),
                is_dir: entry
                    .file_type()
                    .is_ok_and(|file_type| file_type.is_dir() && !file_type.is_symlink()),
                entry,
            })
            .collect(),
        Err(_) => return Vec::new(),
    };
    // Names within one directory are unique, so stable ordering for equal keys
    // cannot affect the rendered result.
    entries.sort_unstable_by(|left, right| left.name.cmp(&right.name));
    entries
}

fn append_tree(
    path: &Path,
    prefix: &mut String,
    depth: usize,
    max_depth: usize,
    output: &mut String,
) {
    if depth >= max_depth {
        return;
    }

    let entries = sorted_entries(path);
    let count = entries.len();
    for (index, tree_entry) in entries.into_iter().enumerate() {
        let is_last = index == count - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let name = tree_entry.name.to_string_lossy();

        output.reserve(
            1 + prefix.len() + connector.len() + name.len() + usize::from(tree_entry.is_dir),
        );
        output.push('\n');
        output.push_str(prefix);
        output.push_str(connector);
        output.push_str(&name);
        if tree_entry.is_dir {
            output.push('/');

            let previous_prefix_len = prefix.len();
            prefix.push_str(if is_last { "    " } else { "│   " });
            append_tree(
                &tree_entry.entry.path(),
                prefix,
                depth + 1,
                max_depth,
                output,
            );
            prefix.truncate(previous_prefix_len);
        }
    }
}

#[cfg(test)]
mod tests;
