//! Purpose: Bounded directory walks that never follow a link, the files they
//! collect, and the broad kind of each file.

use std::fs;
use std::path::{Path, PathBuf};

/// Entries a tree walk will visit before giving up.
///
/// Large enough for a source tree, small enough that a tool call against `/`
/// by mistake returns rather than hanging the agent that made it.
pub const WALK_BUDGET: usize = 200_000;

/// One file found by [`collect_files`].
pub struct CollectedFile {
    /// Where it is, for opening.
    pub path: PathBuf,
    /// Where it is relative to the walk's root, for reporting: an absolute
    /// path in the answer is noise the caller already knows.
    pub relative: String,
    /// Its own name, for classifying by extension.
    pub name: String,
    pub size: u64,
}

/// Every file under `dir`, bounded and not following symlinks.
///
/// Not following them is what keeps `a -> ..` from walking forever; the budget
/// is what keeps a tree without a loop from doing the same.
#[expect(
    clippy::filetype_is_file,
    reason = "only regular files are collected: hashing a FIFO or a device would block or never end"
)]
pub fn collect_files(root: &Path, dir: &Path, out: &mut Vec<CollectedFile>, budget: usize) {
    if out.len() >= budget {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= budget {
            return;
        }
        let Some(file_type) = walkable_type(&entry) else {
            continue;
        };
        let path = entry.path();
        if file_type.is_dir() {
            collect_files(root, &path, out, budget);
        } else if file_type.is_file() {
            let size = entry.metadata().map_or(0, |meta| meta.len());
            out.push(CollectedFile {
                relative: relative_slash_path(root, &path),
                name: entry.file_name().to_string_lossy().into_owned(),
                size,
                path,
            });
        }
    }
}

/// Which broad category a file name falls into.
///
/// By extension, because that is what the caller can see too — sniffing the
/// contents would be more accurate and would also mean opening every file in
/// the tree, which is the one thing a walk must not do.
pub fn content_kind(name: &str) -> &'static str {
    let extension = Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    match extension.as_str() {
        "rs" | "c" | "h" | "cc" | "cpp" | "hpp" | "cs" | "go" | "java" | "js" | "jsx" | "ts"
        | "tsx" | "py" | "rb" | "php" | "kt" | "swift" | "lua" | "sh" | "bash" | "ps1" | "sql"
        | "css" | "scss" | "html" | "htm" | "vue" => "source",
        "md" | "markdown" | "txt" | "rst" | "adoc" | "pdf" | "doc" | "docx" | "odt" | "rtf"
        | "tex" | "epub" => "document",
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" | "ico" | "tif" | "tiff" => "image",
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "opus" | "wma" => "audio",
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "wmv" | "flv" | "m4v" => "video",
        "zip" | "tar" | "gz" | "bz2" | "xz" | "zst" | "7z" | "rar" | "jar" | "whl" => "archive",
        "exe" | "dll" | "so" | "dylib" | "a" | "o" | "bin" | "msi" | "deb" | "rpm" => "program",
        "json" | "yaml" | "yml" | "toml" | "xml" | "csv" | "tsv" | "db" | "sqlite" | "log"
        | "lock" | "parquet" => "data",
        _ => "other",
    }
}

/// The type of an entry a recursive tool may act on, or `None` when the walk
/// must step over it.
///
/// WHY: `safe_path` confines the directory a walk *starts* from; the walk
/// itself reaches every entry below it. A symlink planted inside the root may
/// name any file on the machine, and `File::open`, `read_to_string`,
/// `fs::metadata` and `remove_dir_all` all follow it — so a walker that
/// classifies entries with `file_type()` (which does not follow) and then
/// re-opens them by path reads, or writes, outside the root. Links are
/// therefore never walked; tools that report links (`find`) do so from
/// `symlink_metadata` without opening them. An entry whose type cannot be read
/// is skipped too: fail closed.
pub fn walkable_type(entry: &fs::DirEntry) -> Option<fs::FileType> {
    let file_type = entry.file_type().ok()?;
    (!file_type.is_symlink()).then_some(file_type)
}

/// `path` relative to `root`, always `/`-separated so glob patterns match the
/// same way on every platform.
pub fn relative_slash_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
