//! Purpose: Bounded directory walks that never follow a link, the files they
//! collect, and the broad kind of each file.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[cfg(test)]
mod tests;

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
    pub modified: Option<SystemTime>,
}

/// Every file under `dir`, bounded and not following symlinks.
///
/// Not following them is what keeps `a -> ..` from walking forever; the budget
/// is what keeps a tree without a loop from doing the same.
pub fn collect_files(root: &Path, dir: &Path, out: &mut Vec<CollectedFile>, budget: usize) {
    let mut rel_buf = slash_components(dir.strip_prefix(root).unwrap_or(dir));
    collect_files_into(dir, &mut rel_buf, out, budget);
}

fn collect_files_into(
    dir: &Path,
    rel_buf: &mut String,
    out: &mut Vec<CollectedFile>,
    budget: usize,
) {
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
        collect_entry(&entry, file_type, rel_buf, out, budget);
    }
}

#[expect(
    clippy::filetype_is_file,
    reason = "only regular files are collected: hashing a FIFO or a device would block or never end"
)]
fn collect_entry(
    entry: &fs::DirEntry,
    file_type: fs::FileType,
    rel_buf: &mut String,
    out: &mut Vec<CollectedFile>,
    budget: usize,
) {
    let name = entry.file_name().to_string_lossy().into_owned();
    let base_len = rel_buf.len();
    if base_len > 0 {
        rel_buf.push('/');
    }
    rel_buf.push_str(&name);
    if file_type.is_dir() {
        collect_files_into(&entry.path(), rel_buf, out, budget);
    }
    if file_type.is_file() {
        push_collected_file(entry, rel_buf.clone(), name, out);
    }
    rel_buf.truncate(base_len);
}

fn push_collected_file(
    entry: &fs::DirEntry,
    relative: String,
    name: String,
    out: &mut Vec<CollectedFile>,
) {
    let (size, modified) = entry
        .metadata()
        .map_or((0, None), |meta| (meta.len(), meta.modified().ok()));
    out.push(CollectedFile {
        path: entry.path(),
        relative,
        name,
        size,
        modified,
    });
}

/// Which broad category a file name falls into.
///
/// By extension, because that is what the caller can see too — sniffing the
/// contents would be more accurate and would also mean opening every file in
/// the tree, which is the one thing a walk must not do.
pub fn content_kind(name: &str) -> &'static str {
    let Some(ext) = Path::new(name).extension().and_then(|e| e.to_str()) else {
        return "other";
    };
    let bytes = ext.as_bytes();
    let mut buf = [0u8; 8];
    let Some(dst) = buf.get_mut(..bytes.len()) else {
        return "other";
    };
    for (slot, &src) in dst.iter_mut().zip(bytes) {
        *slot = src.to_ascii_lowercase();
    }
    classify_lower_ext(dst)
}

fn classify_lower_ext(ext: &[u8]) -> &'static str {
    match ext {
        b"rs" | b"c" | b"h" | b"cc" | b"cpp" | b"hpp" | b"cs" | b"go" | b"java" | b"js"
        | b"jsx" | b"ts" | b"tsx" | b"py" | b"rb" | b"php" | b"kt" | b"swift" | b"lua" | b"sh"
        | b"bash" | b"ps1" | b"sql" | b"css" | b"scss" | b"html" | b"htm" | b"vue" => "source",
        b"md" | b"markdown" | b"txt" | b"rst" | b"adoc" | b"pdf" | b"doc" | b"docx" | b"odt"
        | b"rtf" | b"tex" | b"epub" => "document",
        b"png" | b"jpg" | b"jpeg" | b"gif" | b"bmp" | b"svg" | b"webp" | b"ico" | b"tif"
        | b"tiff" => "image",
        b"mp3" | b"wav" | b"flac" | b"ogg" | b"m4a" | b"aac" | b"opus" | b"wma" => "audio",
        b"mp4" | b"mkv" | b"avi" | b"mov" | b"webm" | b"wmv" | b"flv" | b"m4v" => "video",
        b"zip" | b"tar" | b"gz" | b"bz2" | b"xz" | b"zst" | b"7z" | b"rar" | b"jar" | b"whl" => {
            "archive"
        }
        b"exe" | b"dll" | b"so" | b"dylib" | b"a" | b"o" | b"bin" | b"msi" | b"deb" | b"rpm" => {
            "program"
        }
        b"json" | b"yaml" | b"yml" | b"toml" | b"xml" | b"csv" | b"tsv" | b"db" | b"sqlite"
        | b"log" | b"lock" | b"parquet" => "data",
        _ => "other",
    }
}

/// The type of an entry a recursive tool may act on, or `None` when the walk
/// must step over it.
pub fn walkable_type(entry: &fs::DirEntry) -> Option<fs::FileType> {
    let file_type = entry.file_type().ok()?;
    (!file_type.is_symlink()).then_some(file_type)
}

fn slash_components(path: &Path) -> String {
    let mut out = String::new();
    for comp in path.components() {
        if !out.is_empty() {
            out.push('/');
        }
        out.push_str(&comp.as_os_str().to_string_lossy());
    }
    out
}

/// `path` relative to `root`, always `/`-separated so glob patterns match the
/// same way on every platform.
#[cfg(test)]
pub fn relative_slash_path(root: &Path, path: &Path) -> String {
    slash_components(path.strip_prefix(root).unwrap_or(path))
}
