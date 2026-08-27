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
            match entry.file_type() {
                Ok(file_type) if file_type.is_dir() && !file_type.is_symlink() => {
                    pending.push(entry.path());
                }
                // `DirEntry::metadata` does not follow symlinks. On Windows its
                // data is already present in the directory enumeration, avoiding
                // a path allocation and a fresh metadata query for every file.
                Ok(_) => {
                    if let Ok(metadata) = entry.metadata() {
                        bytes += metadata.len();
                    }
                }
                // Preserve the old best-effort behavior on filesystems that
                // cannot report a type but can still return entry metadata.
                Err(_) => {
                    let Ok(metadata) = entry.metadata() else {
                        continue;
                    };
                    if metadata.is_dir() {
                        pending.push(entry.path());
                    } else {
                        bytes += metadata.len();
                    }
                }
            }
        }
    }
    bytes
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
mod tests {
    use super::{directory_size, render_directory_tree};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn directory_size_totals_nested_files() {
        let temp = TempDir::new().expect("temp dir");
        fs::create_dir_all(temp.path().join("nested")).expect("nested dir");
        fs::write(temp.path().join("root.bin"), b"1234").expect("root file");
        fs::write(temp.path().join("nested/child.bin"), b"123456").expect("child file");

        assert_eq!(directory_size(temp.path()), 10);
    }

    #[test]
    fn render_directory_tree_sorts_entries_and_honours_depth() {
        let temp = TempDir::new().expect("temp dir");
        fs::create_dir_all(temp.path().join("alpha/deeper")).expect("nested dirs");
        fs::write(temp.path().join("zeta.txt"), b"z").expect("file");
        fs::write(temp.path().join("alpha/child.txt"), b"a").expect("child file");

        let tree = render_directory_tree(temp.path(), 1);
        let rows: Vec<_> = tree.lines().skip(1).collect();

        assert_eq!(rows, ["├── alpha/", "└── zeta.txt"]);
    }

    #[test]
    fn render_directory_tree_reuses_the_correct_nested_prefix() {
        let temp = TempDir::new().expect("temp dir");
        fs::create_dir_all(temp.path().join("alpha/deeper")).expect("nested dirs");
        fs::write(temp.path().join("alpha/child.txt"), b"a").expect("child file");
        fs::write(temp.path().join("zeta.txt"), b"z").expect("root file");

        let tree = render_directory_tree(temp.path(), 2);
        let rows: Vec<_> = tree.lines().skip(1).collect();

        assert_eq!(
            rows,
            [
                "├── alpha/",
                "│   ├── child.txt",
                "│   └── deeper/",
                "└── zeta.txt"
            ]
        );
    }

    #[test]
    fn missing_paths_have_no_size_and_only_render_the_root() {
        let temp = TempDir::new().expect("temp dir");
        let missing = temp.path().join("missing");

        assert_eq!(directory_size(&missing), 0);
        assert_eq!(
            render_directory_tree(&missing, usize::MAX),
            format!("{}/", missing.display())
        );
    }

    #[cfg(unix)]
    #[test]
    fn directory_size_counts_a_link_without_following_it() {
        let root = TempDir::new().expect("root temp dir");
        let outside = TempDir::new().expect("outside temp dir");
        fs::write(outside.path().join("large.bin"), vec![0_u8; 64 * 1024]).expect("outside file");
        std::os::unix::fs::symlink(outside.path(), root.path().join("link"))
            .expect("directory link");

        assert!(directory_size(root.path()) < 64 * 1024);
    }
}
