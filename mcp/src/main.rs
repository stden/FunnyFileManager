//! FFM MCP Server — Model Context Protocol server for FunnyFileManager.
//!
//! Exposes file management operations as MCP tools so that AI assistants
//! (Claude, ChatGPT, etc.) can manage files, directories, and S3 objects.
//!
//! Built on the official `rmcp` SDK with stdio transport.

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
    ErrorData as McpError, ServerHandler, ServiceExt,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write as IoWrite};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime};
use tracing::info;

use ffm_mcp::tree_walk::{directory_size, render_directory_tree};

mod cleanup;
mod content_search;
mod directory_tools;
mod fsops;
mod globmatch;
mod kdbx;
mod paths;
mod pattern_search;
mod replace;
mod reports;
mod s3_profiles;
// The tool list lives in its own import-free file so the sibling crate's MCP
// status screen can `include!` it instead of keeping a second copy.
mod tool_names;
pub use tool_names::TOOL_NAMES;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_conversions;
#[cfg(test)]
mod tests_kdbx;
#[cfg(test)]
mod tests_tools;

// ─── Utility ────────────────────────────────────────────────────

fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;
    if size >= GB {
        format!("{} GB", two_decimals(size, GB))
    } else if size >= MB {
        format!("{} MB", two_decimals(size, MB))
    } else if size >= KB {
        format!("{} KB", two_decimals(size, KB))
    } else {
        format!("{size} B")
    }
}

/// `size / unit` to two decimals, rounded half to even as `{:.2}` rounds.
///
/// Integer maths in `u128`: exact for every `u64`, where an `f64` quotient
/// would lose the low bits of any size above 2^53.
fn two_decimals(size: u64, unit: u64) -> String {
    let unit = u128::from(unit);
    let scaled = u128::from(size) * 100;
    let (mut hundredths, remainder) = (scaled / unit, scaled % unit);
    if remainder * 2 > unit || (remainder * 2 == unit && hundredths % 2 == 1) {
        hundredths += 1;
    }
    format!("{}.{:02}", hundredths / 100, hundredths % 100)
}

/// A client-supplied `u64` count as a `usize`, saturating where `usize` is
/// narrower: a line offset or limit past `usize::MAX` means "all of them".
fn saturating_usize(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

/// An S3 object size as bytes. The SDK models it as `i64`; a missing or
/// negative size reports as zero rather than wrapping to 16 EiB.
fn object_size(size: Option<i64>) -> u64 {
    size.and_then(|bytes| u64::try_from(bytes).ok())
        .unwrap_or(0)
}

/// Copy one entry, preserving what it *is* — the single place that decides how
/// a source is transferred, so every copy path agrees.
///
/// WHY `symlink_metadata` and never `is_dir()`/`exists()`: those follow the
/// link. Copying through one would pull in the bytes it points at — content
/// from outside the server root, landing inside it — and a link to a directory
/// would drag the whole target tree along. Reproducing the link as a link is
/// also what keeps a dangling link copyable at all.
fn copy_entry(src: &Path, dst: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(src)?;
    if meta.is_symlink() {
        return copy_symlink(src, dst);
    }
    if meta.is_dir() {
        return copy_dir_recursive(src, dst);
    }
    fs::copy(src, dst).map(|_| ())
}

/// Recreate `src` at `dst` as a link to the same target, without reading it.
#[cfg(unix)]
fn copy_symlink(src: &Path, dst: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(fs::read_link(src)?, dst)
}

/// Same, for Windows — where the link itself carries the file/directory
/// distinction, so the kind has to be chosen from the target.
#[cfg(windows)]
fn copy_symlink(src: &Path, dst: &Path) -> io::Result<()> {
    let target = fs::read_link(src)?;
    if target.is_dir() {
        std::os::windows::fs::symlink_dir(&target, dst)
    } else {
        std::os::windows::fs::symlink_file(&target, dst)
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        copy_entry(&entry.path(), &dst.join(entry.file_name()))?;
    }
    Ok(())
}

/// Entries a tree walk will visit before giving up.
///
/// Large enough for a source tree, small enough that a tool call against `/`
/// by mistake returns rather than hanging the agent that made it.
const WALK_BUDGET: usize = 200_000;
/// How far `tail_file` will read backwards. A file with no newlines would
/// otherwise be slurped whole, which is the opposite of seeking from the end.
const MAX_TAIL_BYTES: usize = 10 * 1024 * 1024;
/// Bytes `read_line_range` will consume before it gives up, and the largest
/// file `view_file` accepts. Without the cap, a `read_lines` call with no `to`
/// — or a file with no newlines — would buffer without bound.
const MAX_READ_BYTES: u64 = 10 * 1024 * 1024;
/// A JSON `days` value is unbounded; the window is capped at a century.
const MAX_RECENT_DAYS: u64 = 36_500;

/// One file found by [`collect_files`].
struct CollectedFile {
    /// Where it is, for opening.
    path: PathBuf,
    /// Where it is relative to the walk's root, for reporting: an absolute
    /// path in the answer is noise the caller already knows.
    relative: String,
    /// Its own name, for classifying by extension.
    name: String,
    size: u64,
}

/// Every file under `dir`, bounded and not following symlinks.
///
/// Not following them is what keeps `a -> ..` from walking forever; the budget
/// is what keeps a tree without a loop from doing the same.
fn collect_files(root: &Path, dir: &Path, out: &mut Vec<CollectedFile>, budget: usize) {
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
fn content_kind(name: &str) -> &'static str {
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

/// A file's SHA-256, or `None` if it cannot be read.
fn checksum_of(path: &Path) -> Option<String> {
    checksum_file(path).ok()
}

/// How much of a file `text_format` looks at. The punctuation of a text file is
/// established in its first pages; reading a gigabyte to confirm it would make
/// the cheap question expensive.
const MAX_FORMAT_BYTES: usize = 256 * 1024;

/// `count` lines starting at `start` (1-based), and whether the read consumed
/// the file to its end.
///
/// Reads no further than it must: `head_file` and a window near the front of a
/// large file both stop as soon as they have what they came for. Gives up past
/// [`MAX_READ_BYTES`]: a request with no end — or a file with no newlines —
/// would otherwise buffer the whole file, which no other reader here does.
fn read_line_range(path: &Path, start: usize, count: usize) -> io::Result<(Vec<String>, bool)> {
    use io::BufRead;

    let mut lines = Vec::new();
    if count == 0 {
        return Ok((lines, false));
    }
    // One byte past the cap, so a request satisfied at the cap exactly is
    // distinguishable from one that needed more than it.
    let mut reader = io::BufReader::new(fs::File::open(path)?).take(MAX_READ_BYTES + 1);
    for (number, line) in reader.by_ref().lines().enumerate() {
        let number = number + 1;
        if number < start {
            continue;
        }
        lines.push(line?);
        if lines.len() == count {
            break;
        }
    }
    if reader.limit() == 0 {
        return Err(io::Error::other(format!(
            "gave up after reading {}: narrow the range, or use tail_file for the end",
            format_size(MAX_READ_BYTES)
        )));
    }
    // `reached_end` must mean "the file was consumed to EOF", even when the
    // count was satisfied on the file's last line — a paging client that is
    // told `false` there asks for a page past the end.
    let reached_end = reader.fill_buf()?.is_empty();
    Ok((lines, reached_end))
}

/// At most `limit` bytes from the front of `path`.
fn read_capped(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
    use io::Read;

    let mut buffer = Vec::new();
    fs::File::open(path)?
        .take(limit as u64)
        .read_to_end(&mut buffer)?;
    Ok(buffer)
}

/// The last byte of `path`, or `None` for an empty file. One seek and one
/// read: the end of a file is never worth streaming to.
fn last_byte(path: &Path) -> io::Result<Option<u8>> {
    let mut file = fs::File::open(path)?;
    if file.seek(SeekFrom::End(0))? == 0 {
        return Ok(None);
    }
    file.seek(SeekFrom::End(-1))?;
    let mut byte = [0u8; 1];
    file.read_exact(&mut byte)?;
    Ok(Some(byte[0]))
}

/// What a text file's punctuation is, which an edit has to preserve.
struct TextFormat {
    endings: &'static str,
    crlf: usize,
    lf: usize,
    cr: usize,
    final_newline: bool,
    bom: &'static str,
    binary: bool,
}

impl TextFormat {
    fn of(bytes: &[u8]) -> Self {
        let bom = match bytes {
            [0xEF, 0xBB, 0xBF, ..] => "utf-8",
            [0xFF, 0xFE, ..] => "utf-16le",
            [0xFE, 0xFF, ..] => "utf-16be",
            _ => "none",
        };
        // A CR is only a lone CR when no LF follows it, which is the same rule
        // that makes a CRLF one ending rather than two.
        let crlf = bytes.windows(2).filter(|pair| pair == b"\r\n").count();
        let lf = bytes.iter().filter(|byte| **byte == b'\n').count() - crlf;
        let cr = bytes.iter().filter(|byte| **byte == b'\r').count() - crlf;
        let endings = match (crlf > 0, lf > 0, cr > 0) {
            (true, false, false) => "crlf",
            (false, true, false) => "lf",
            (false, false, true) => "cr",
            (false, false, false) => "none",
            _ => "mixed",
        };
        Self {
            endings,
            crlf,
            lf,
            cr,
            final_newline: bytes.last() == Some(&b'\n'),
            bom,
            // A NUL byte is what every editor uses to decide the same thing:
            // no text encoding this server can report puts one in the middle.
            binary: bytes.contains(&0),
        }
    }
}

/// The offset of the first byte at which two files differ, or `None` when they
/// are identical. Streams both rather than loading either.
///
/// When one file is a prefix of the other, the difference is at the shorter
/// file's length — the first offset where only one of them still has a byte.
fn first_difference(left: &Path, right: &Path) -> io::Result<Option<u64>> {
    use io::BufRead;

    const BLOCK: usize = 64 * 1024;
    let mut left = io::BufReader::with_capacity(BLOCK, fs::File::open(left)?);
    let mut right = io::BufReader::with_capacity(BLOCK, fs::File::open(right)?);
    let mut offset = 0u64;

    loop {
        let left_block = left.fill_buf()?;
        let right_block = right.fill_buf()?;
        if left_block.is_empty() && right_block.is_empty() {
            return Ok(None);
        }
        if left_block.is_empty() || right_block.is_empty() {
            return Ok(Some(offset));
        }
        let step = left_block.len().min(right_block.len());
        if let Some(at) = left_block[..step]
            .iter()
            .zip(&right_block[..step])
            .position(|(a, b)| a != b)
        {
            return Ok(Some(offset + at as u64));
        }
        left.consume(step);
        right.consume(step);
        offset += step as u64;
    }
}

/// Every `name` in `path_list` (a PATH-style value), in order, so a shadowed
/// executable is visible.
///
/// On Windows a bare name is tried against each PATHEXT suffix as well, which
/// is how the shell itself resolves `cargo` to `cargo.exe`.
fn which_all(name: &str, path_list: &std::ffi::OsStr) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for directory in std::env::split_paths(path_list) {
        for candidate in candidate_names(name) {
            let file = directory.join(&candidate);
            if file.is_file() && is_executable(&file) && !found.contains(&file) {
                found.push(file);
            }
        }
    }
    found
}

/// Whether `path` carries any execute bit. That is a permission-bit test,
/// not an access(X_OK) answer for the current user — but without it a plain
/// file named like the command — a README, a data file — shadows the real
/// binary, and `which` names something no shell would ever run.
#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).is_ok_and(|meta| meta.permissions().mode() & 0o111 != 0)
}

/// Windows resolves a command by its PATHEXT suffix, not by a permission bit.
#[cfg(windows)]
fn is_executable(_path: &Path) -> bool {
    true
}

/// The file names a bare command could have on this platform.
fn candidate_names(name: &str) -> Vec<String> {
    use std::env;
    if !cfg!(windows) {
        return vec![name.to_owned()];
    }
    let extensions = env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_owned());
    let mut names = vec![name.to_owned()];
    names.extend(
        extensions
            .split(';')
            .map(str::trim)
            .filter(|extension| !extension.is_empty())
            .map(|extension| format!("{name}{extension}")),
    );
    names
}

/// SHA-256 of `path`, streaming so a large duplicate costs a read, not a copy.
fn checksum_file(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// The last `wanted` lines of a file, and its size.
///
/// Reads backwards in blocks until enough newlines have gone by, so tailing a
/// gigabyte log costs the same as tailing a small one. A file smaller than one
/// block is simply read whole.
fn tail_lines(path: &Path, wanted: usize) -> io::Result<(Vec<String>, u64)> {
    let mut file = fs::File::open(path)?;
    let size = file.seek(SeekFrom::End(0))?;
    let wanted = wanted.min(10_000);
    if wanted == 0 {
        return Ok((Vec::new(), size));
    }

    let buffer = read_tail_bytes(&mut file, size, wanted)?;

    let text = String::from_utf8_lossy(&buffer);
    let mut lines: Vec<String> = text.lines().map(ToOwned::to_owned).collect();
    if lines.len() > wanted {
        lines.drain(..lines.len() - wanted);
    }
    Ok((lines, size))
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
fn walkable_type(entry: &fs::DirEntry) -> Option<fs::FileType> {
    let file_type = entry.file_type().ok()?;
    (!file_type.is_symlink()).then_some(file_type)
}

fn search_recursive(
    dir: &Path,
    pattern: &str,
    files_only: bool,
    dirs_only: bool,
    results: &mut Vec<serde_json::Value>,
    max: usize,
) {
    if results.len() >= max {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        if results.len() >= max {
            return;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

        if name.to_lowercase().contains(pattern) {
            let include = (!files_only || !is_dir) && (!dirs_only || is_dir);
            if include {
                results.push(json!({
                    "path": entry.path().display().to_string(),
                    "name": name,
                    "type": if is_dir { "dir" } else { "file" },
                }));
            }
        }
        if is_dir {
            search_recursive(&entry.path(), pattern, files_only, dirs_only, results, max);
        }
    }
}

/// `path` relative to `root`, always `/`-separated so glob patterns match the
/// same way on every platform.
fn relative_slash_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Join a caller-supplied name onto a directory that is already confined to
/// the server root — the only sanctioned way to build a path from an argument
/// the sandbox never inspected.
///
/// WHY: `Path::join` is not a string append. An absolute operand *replaces* the
/// base outright, and `..` components walk back out of it, so joining a raw
/// name onto a confined directory throws the confinement away:
/// `join(root, "../../../../etc/passwd")` lands on `/etc/passwd`, and a caller
/// who cannot read that file can still learn whether it exists from whatever
/// the tool reports back. Exactly one ordinary component may be joined.
fn join_child(base: &Path, name: &str) -> Result<PathBuf, String> {
    let mut components = Path::new(name).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(single)), None) => Ok(base.join(single)),
        _ => Err(format!(
            "'{name}' is not a plain file name: exactly one path component is allowed"
        )),
    }
}

/// Generate a unique file path to avoid overwriting existing files.
/// If "test.txt" already exists, returns "test_1.txt", "test_2.txt", etc.
fn unique_destination(base: &Path, file_name: &str) -> Result<PathBuf, String> {
    // The counter suffixes below are derived from this one validated component,
    // so `{stem}_{n}[.{ext}]` can never grow a separator or a `..` of its own.
    let mut dest = join_child(base, file_name)?;
    if !dest.exists() {
        return Ok(dest);
    }

    let path = Path::new(file_name);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let extension = path.extension().and_then(|e| e.to_str());

    const MAX_ATTEMPTS: u32 = 9999;
    for counter in 1..=MAX_ATTEMPTS {
        let new_name = match extension {
            Some(ext) if !ext.is_empty() => format!("{}_{}.{}", stem, counter, ext),
            _ => format!("{}_{}", stem, counter),
        };
        dest = base.join(new_name);
        if !dest.exists() {
            return Ok(dest);
        }
    }
    Err(format!(
        "could not find unique name for '{}' after {} attempts",
        file_name, MAX_ATTEMPTS
    ))
}

/// The final component of a resolved path, as the batch tools report it and as
/// they name the copy at the destination.
fn entry_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

/// Bookmarks storage path: ~/.ffm/bookmarks.json
fn bookmarks_path() -> Option<PathBuf> {
    dirs_home().map(|h| h.join(".ffm").join("bookmarks.json"))
}

fn load_bookmarks() -> serde_json::Map<String, serde_json::Value> {
    bookmarks_path()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_bookmarks(bookmarks: &serde_json::Map<String, serde_json::Value>) -> Result<(), String> {
    let path = bookmarks_path().ok_or("cannot determine home directory")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(bookmarks).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

// ─── Request Parameter Types ────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct AnalyseParams {
    /// Absolute path to the directory to measure
    path: String,
    /// How many of the largest files to name (default 10)
    #[serde(default)]
    largest: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DuplicatesParams {
    /// Absolute path to the directory to search
    path: String,
    /// Ignore files smaller than this many bytes (default 1)
    #[serde(default)]
    min_size: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TailParams {
    /// Absolute path to the file
    path: String,
    /// How many lines from the end (default 20)
    #[serde(default)]
    lines: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct HeadParams {
    /// Absolute path to the file
    path: String,
    /// How many lines from the start (default 20)
    #[serde(default)]
    lines: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct LineRangeParams {
    /// Absolute path to the file
    path: String,
    /// First line to return, 1-based and inclusive (default 1)
    #[serde(default)]
    from: Option<usize>,
    /// Last line to return, 1-based and inclusive (default: to the end)
    #[serde(default)]
    to: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CompareFilesParams {
    /// Absolute path to the first file
    left: String,
    /// Absolute path to the second file
    right: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct LargestParams {
    /// Absolute path to the directory to search
    path: String,
    /// How many files to report (default 20)
    #[serde(default)]
    limit: Option<usize>,
    /// Ignore files smaller than this many bytes (default 1)
    #[serde(default)]
    min_size: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct WhichParams {
    /// Executable name to look for, without a directory component
    name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CompareParams {
    /// Absolute path to the first directory
    left: String,
    /// Absolute path to the second directory
    right: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct RecentParams {
    /// Absolute path to the directory to search
    path: String,
    /// Only files modified within this many days (default 7)
    #[serde(default)]
    days: Option<u64>,
    /// How many to return, newest first (default 50)
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PathParam {
    /// Absolute path to a file or directory
    path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ViewFileParams {
    /// Absolute path to the file to view
    path: String,
    /// Line offset to start reading from (0-based, default 0)
    #[serde(default)]
    offset: Option<u64>,
    /// Maximum number of lines to return (default: all)
    #[serde(default)]
    limit: Option<u64>,
}

/// Output representation for the `kbdx` tool.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
enum KdbxFormat {
    /// Pretty, deterministic JSON containing the complete decrypted model.
    #[default]
    Json,
    /// YAML containing the same complete decrypted model as JSON.
    Yaml,
    /// Dotenv variables for current entry passwords only.
    Env,
}

impl From<KdbxFormat> for kdbx::OutputFormat {
    fn from(value: KdbxFormat) -> Self {
        match value {
            KdbxFormat::Json => Self::Json,
            KdbxFormat::Yaml => Self::Yaml,
            KdbxFormat::Env => Self::Env,
        }
    }
}

#[derive(Deserialize, schemars::JsonSchema)]
struct KdbxParams {
    /// Path to an existing `.kdbx` database; common `.kbdx` transposition accepted.
    path: String,
    /// Master password. Omit only for a key-file-only database. Never logged.
    #[serde(default)]
    password: Option<String>,
    /// Optional KeePass key file; it must also be inside the allowed root.
    #[serde(default)]
    keyfile: Option<String>,
    /// Output format: `json` (default), `yaml`, or password-only `env`.
    #[serde(default)]
    format: KdbxFormat,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CreateFileParams {
    /// Path for the new file
    path: String,
    /// File content (default: empty)
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct WriteFileParams {
    /// Path to the file
    path: String,
    /// Content to write
    content: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CopyMoveParams {
    /// Source path
    source: String,
    /// Destination path
    destination: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DeleteParams {
    /// Path to delete
    path: String,
    /// Allow recursive directory deletion (default: true)
    #[serde(default = "default_true")]
    recursive: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchParams {
    /// Root directory to search in
    path: String,
    /// Name pattern to match (case-insensitive substring)
    pattern: String,
    /// Maximum results to return (default: 100)
    #[serde(default = "default_max_results")]
    max_results: usize,
    /// Only return files
    #[serde(default)]
    files_only: bool,
    /// Only return directories
    #[serde(default)]
    dirs_only: bool,
}

fn default_max_results() -> usize {
    100
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TreeParams {
    /// Root directory
    path: String,
    /// Max depth (default: 3)
    #[serde(default = "default_depth")]
    depth: usize,
}

fn default_depth() -> usize {
    3
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SortDirectoryParams {
    /// Directory path to list
    path: String,
    /// Sort mode: "name" (default), "extension", or "size"
    #[serde(default = "default_sort_mode")]
    sort: String,
}

fn default_sort_mode() -> String {
    "name".to_owned()
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct UniqueNameParams {
    /// Directory where the file will be created
    directory: String,
    /// Desired filename (e.g. "report.txt")
    filename: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct BatchParams {
    /// List of source paths
    paths: Vec<String>,
    /// Destination directory (for copy/move)
    #[serde(default)]
    destination: Option<String>,
    /// Allow recursive directory operations (default: true)
    #[serde(default = "default_true")]
    recursive: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SymlinkParams {
    /// Path where the symlink will be created
    link: String,
    /// Target path the symlink points to
    target: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PermissionsParams {
    /// Path to the file or directory
    path: String,
    /// Unix permission mode as octal string (e.g. "755", "644")
    #[serde(default)]
    mode: Option<String>,
    /// Set read-only flag (cross-platform)
    #[serde(default)]
    readonly: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct BookmarkParams {
    /// Bookmark name/key
    name: String,
    /// Path to bookmark (required for set_bookmark)
    #[serde(default)]
    path: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct S3ProfileParam {
    /// AWS profile name from ~/.aws/credentials
    profile: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct S3ListObjectsParams {
    /// AWS profile name
    profile: String,
    /// S3 bucket name
    bucket: String,
    /// Prefix/folder to list (default: root)
    #[serde(default)]
    prefix: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct S3TransferParams {
    /// AWS profile name
    profile: String,
    /// S3 bucket name
    bucket: String,
    /// S3 object key
    key: String,
    /// Local file path
    local_path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct S3DeleteParams {
    /// AWS profile name
    profile: String,
    /// S3 bucket name
    bucket: String,
    /// S3 object key to delete
    key: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct S3CopyParams {
    /// AWS profile name
    profile: String,
    /// Source bucket
    source_bucket: String,
    /// Source object key
    source_key: String,
    /// Destination bucket
    dest_bucket: String,
    /// Destination object key
    dest_key: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GrepParams {
    /// Directory to search in
    path: String,
    /// Text pattern to search for (case-insensitive)
    pattern: String,
    /// File extension filter (e.g. "rs", "txt")
    #[serde(default)]
    extension: Option<String>,
    /// Maximum results (default: 50)
    #[serde(default = "default_grep_max")]
    max_results: usize,
}

fn default_grep_max() -> usize {
    50
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct AppendFileParams {
    /// Path to the file
    path: String,
    /// Content to append
    content: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GlobParams {
    /// Root directory to search in
    path: String,
    /// Glob pattern (e.g. "*.rs", "**/*.txt", "src/**/*.json")
    pattern: String,
    /// Maximum results (default: 100)
    #[serde(default = "default_max_results")]
    max_results: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct FindParams {
    /// Root directory to search in
    path: String,
    /// Name glob matched against the entry name (e.g. "*.rs", "mod?.rs").
    /// Without a wildcard it must equal the name — optional
    #[serde(default)]
    name: Option<String>,
    /// Filter by type: "file", "dir", or "symlink" — optional
    #[serde(default)]
    file_type: Option<String>,
    /// Minimum file size in bytes — optional
    #[serde(default)]
    min_size: Option<u64>,
    /// Maximum file size in bytes — optional
    #[serde(default)]
    max_size: Option<u64>,
    /// Maximum directory depth to search (default: unlimited)
    #[serde(default)]
    max_depth: Option<usize>,
    /// Only files modified within the last N seconds — optional
    #[serde(default)]
    modified_within_secs: Option<u64>,
    /// Maximum results (default: 100)
    #[serde(default = "default_max_results")]
    max_results: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ReplaceParams {
    /// Root directory to search in
    path: String,
    /// Text to find (case-sensitive)
    find: String,
    /// Text to replace with
    replace: String,
    /// File extension filter (e.g. "rs", "txt") — optional
    #[serde(default)]
    extension: Option<String>,
    /// Glob pattern for filenames (e.g. "*.rs") — optional
    #[serde(default)]
    name_pattern: Option<String>,
    /// Perform a dry run without modifying files (default: false)
    #[serde(default)]
    dry_run: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DeleteByPatternParams {
    /// Root directory to search in
    path: String,
    /// Glob pattern for files to delete (e.g. "*.tmp", "*.bak")
    pattern: String,
    /// Also delete matching directories (default: false)
    #[serde(default)]
    include_dirs: bool,
    /// Recursive — search subdirectories (default: true)
    #[serde(default = "default_true")]
    recursive: bool,
    /// Perform a dry run without deleting (default: false)
    #[serde(default)]
    dry_run: bool,
}

// ─── S3 Client Helper ───────────────────────────────────────────

async fn make_s3_client(profile: &str) -> Result<aws_sdk_s3::Client, McpError> {
    use aws_config::BehaviorVersion;

    let config = aws_config::defaults(BehaviorVersion::latest())
        .credentials_provider(
            aws_config::profile::ProfileFileCredentialsProvider::builder()
                .profile_name(profile)
                .build(),
        )
        .region(
            aws_config::meta::region::RegionProviderChain::default_provider().or_else("us-east-1"),
        )
        .load()
        .await;

    Ok(aws_sdk_s3::Client::new(&config))
}

// ─── MCP Server ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FfmServer {
    tool_router: ToolRouter<Self>,
    /// The only directory tree tools may touch, resolved once at startup.
    root: PathBuf,
}

impl FfmServer {
    /// Resolve an existing caller-supplied path, confined to the server root.
    fn safe_path(&self, raw: &str) -> Result<PathBuf, McpError> {
        paths::sanitize_path(&self.root, raw).map_err(|e| McpError::invalid_params(e, None))
    }

    fn safe_dir(&self, raw: &str) -> Result<PathBuf, McpError> {
        let path = self.safe_path(raw)?;
        if !path.is_dir() {
            return Err(McpError::invalid_params(
                format!("not a directory: {}", path.display()),
                None,
            ));
        }
        Ok(path)
    }

    fn safe_file(&self, raw: &str) -> Result<PathBuf, McpError> {
        let path = self.safe_path(raw)?;
        if !path.is_file() {
            return Err(McpError::invalid_params(
                format!("not a file: {}", path.display()),
                None,
            ));
        }
        Ok(path)
    }

    /// Same, for paths that do not exist yet (create/write destinations).
    fn safe_path_create(&self, raw: &str) -> Result<PathBuf, McpError> {
        paths::sanitize_path_create(&self.root, raw).map_err(|e| McpError::invalid_params(e, None))
    }

    /// Same, but keeps a final symlink unresolved — for operations that must
    /// act on the link itself rather than on its target.
    fn safe_path_nofollow(&self, raw: &str) -> Result<PathBuf, McpError> {
        paths::sanitize_path_nofollow(&self.root, raw)
            .map_err(|e| McpError::invalid_params(e, None))
    }
}

#[tool_router]
impl FfmServer {
    pub fn new(root: PathBuf) -> Self {
        Self {
            tool_router: Self::tool_router(),
            root,
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // LOCAL FILESYSTEM — Core Operations
    // ═══════════════════════════════════════════════════════════════

    /// List files and directories at the given path.
    /// Returns name, type (file/dir), and size for each entry.
    #[tool(
        description = "List files and directories at the given path. Returns name, type, and size."
    )]
    fn list_directory(
        &self,
        Parameters(params): Parameters<PathParam>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_dir(&params.path)?;

        let mut entries = Vec::new();
        for entry in
            fs::read_dir(&path).map_err(|e| McpError::internal_error(e.to_string(), None))?
        {
            let entry = entry.map_err(|e| McpError::internal_error(e.to_string(), None))?;
            let meta = entry
                .metadata()
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            let name = entry.file_name().to_string_lossy().to_string();
            entries.push(json!({
                "name": name,
                "type": if meta.is_dir() { "dir" } else { "file" },
                "size": meta.len(),
                "size_human": format_size(meta.len()),
            }));
        }

        sort_listing(&mut entries);

        let result = json!({
            "path": path.display().to_string(),
            "count": entries.len(),
            "entries": entries,
        });
        Ok(json_response(&result))
    }

    /// List directory sorted by name, extension, or size (like Panel sort modes).
    #[tool(
        description = "List directory sorted by name, extension, or size. Sort modes: 'name', 'extension', 'size'."
    )]
    fn sort_directory(
        &self,
        Parameters(params): Parameters<SortDirectoryParams>,
    ) -> Result<CallToolResult, McpError> {
        directory_tools::sort_directory(self, &params)
    }

    /// Get detailed info about a file or directory.
    #[tool(
        description = "Get detailed info about a file or directory: size, type, permissions, modified time."
    )]
    fn get_file_info(
        &self,
        Parameters(params): Parameters<PathParam>,
    ) -> Result<CallToolResult, McpError> {
        // Keep a final symlink unresolved, otherwise a link is reported as its
        // target: type "file", the target's path, and no symlink_target field.
        let path = self.safe_path_nofollow(&params.path)?;
        let meta = fs::symlink_metadata(&path)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let file_type = if meta.is_symlink() {
            "symlink"
        } else if meta.is_dir() {
            "directory"
        } else {
            "file"
        };

        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());

        let mut info = json!({
            "path": path.display().to_string(),
            "type": file_type,
            "size": meta.len(),
            "size_human": format_size(meta.len()),
            "readonly": meta.permissions().readonly(),
        });

        append_file_metadata(&mut info, &path, &meta, modified);

        Ok(json_response(&info))
    }

    /// Read and return the contents of a text file. Max 10 MB.
    #[tool(description = "Read and return the contents of a text file. Max 10 MB.")]
    fn view_file(
        &self,
        Parameters(params): Parameters<ViewFileParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_path(&params.path)?;

        if path.is_dir() {
            return Err(McpError::invalid_params("cannot view a directory", None));
        }

        let meta =
            fs::metadata(&path).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        if meta.len() > MAX_READ_BYTES {
            return Err(McpError::invalid_params(
                format!("file too large: {} (max 10 MB)", format_size(meta.len())),
                None,
            ));
        }

        let content =
            fs::read_to_string(&path).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        let offset = params.offset.map_or(0, saturating_usize);
        let limit = params.limit.map_or(total_lines, saturating_usize);

        let selected: Vec<&str> = lines.into_iter().skip(offset).take(limit).collect();

        let result = json!({
            "path": path.display().to_string(),
            "total_lines": total_lines,
            "offset": offset,
            "lines_returned": selected.len(),
            "content": selected.join("\n"),
        });
        Ok(json_response(&result))
    }

    /// Decrypt a KeePass database and return its contents as JSON, YAML, or dotenv text.
    /// JSON/YAML contain the complete model, including plaintext protected values;
    /// dotenv contains current password fields only. Nothing is written to disk.
    #[tool(
        description = "Decrypt a .kdbx/.kbdx KeePass database and return it as JSON (default), YAML, or password-only dotenv text. The response contains plaintext secrets. The database and optional key file must be inside the allowed root; no output file is written.",
        annotations(
            title = "Export KeePass database",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn kbdx(&self, Parameters(params): Parameters<KdbxParams>) -> Result<CallToolResult, McpError> {
        if params.password.is_none() && params.keyfile.is_none() {
            return Err(McpError::invalid_params(
                "provide a password, a key file, or both",
                None,
            ));
        }

        let path = self.safe_file(&params.path)?;
        validate_vault_extension(&path)?;

        let keyfile = params
            .keyfile
            .as_deref()
            .map(|raw| self.safe_path(raw))
            .transpose()?;
        if keyfile.as_ref().is_some_and(|path| !path.is_file()) {
            return Err(McpError::invalid_params(
                "KeePass key file path is not a file",
                None,
            ));
        }

        let output = kdbx::export(
            &path,
            params.password.as_deref(),
            keyfile.as_deref(),
            params.format.into(),
        )
        .map_err(export_error)?;

        Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
    }

    /// Create a new file with the given content. Fails if file already exists.
    #[tool(description = "Create a new file with the given content. Fails if file already exists.")]
    fn create_file(
        &self,
        Parameters(params): Parameters<CreateFileParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_path_create(&params.path)?;
        let content = params.content.as_deref().unwrap_or("");

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        // Atomic create — fails if file already exists (no TOCTOU race)
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| {
                if e.kind() == io::ErrorKind::AlreadyExists {
                    McpError::invalid_params(
                        format!("file already exists: {}", path.display()),
                        None,
                    )
                } else {
                    McpError::internal_error(e.to_string(), None)
                }
            })?;
        file.write_all(content.as_bytes())
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({ "created": path.display().to_string(), "size": content.len() });
        Ok(json_response(&result))
    }

    /// Write content to a file, creating or overwriting it.
    #[tool(description = "Write content to a file, creating or overwriting it.")]
    fn write_file(
        &self,
        Parameters(params): Parameters<WriteFileParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_path_create(&params.path)?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        fs::write(&path, &params.content)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({ "written": path.display().to_string(), "size": params.content.len() });
        Ok(json_response(&result))
    }

    /// Append content to an existing file.
    #[tool(
        description = "Append content to an existing file. Creates the file if it doesn't exist."
    )]
    fn append_file(
        &self,
        Parameters(params): Parameters<AppendFileParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_path_create(&params.path)?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        file.write_all(params.content.as_bytes())
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result =
            json!({ "appended": path.display().to_string(), "bytes_added": params.content.len() });
        Ok(json_response(&result))
    }

    /// Create a new directory (and parent directories if needed).
    #[tool(description = "Create a new directory (and parent directories if needed).")]
    fn create_directory(
        &self,
        Parameters(params): Parameters<PathParam>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_path_create(&params.path)?;

        fs::create_dir_all(&path).map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let canonical = path
            .canonicalize()
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let result = json!({ "created": canonical.display().to_string() });
        Ok(json_response(&result))
    }

    /// Copy a file or directory to a destination. Directories are copied recursively.
    #[tool(
        description = "Copy a file or directory to a destination. Directories are copied recursively."
    )]
    fn copy(
        &self,
        Parameters(params): Parameters<CopyMoveParams>,
    ) -> Result<CallToolResult, McpError> {
        // Resolve the link itself, never its target: canonicalizing would hand
        // back what the link points at, so copying a link would copy that file's
        // contents instead of reproducing the link.
        let source = self.safe_path_nofollow(&params.source)?;
        let dest = self.safe_path_create(&params.destination)?;

        // `symlink_metadata`, not `exists()`: `exists()` follows the link, so a
        // dangling one would be reported as missing although it is right there.
        if fs::symlink_metadata(&source).is_err() {
            return Err(McpError::invalid_params(
                format!("source not found: {}", source.display()),
                None,
            ));
        }

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }
        copy_entry(&source, &dest).map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result =
            json!({ "copied": source.display().to_string(), "to": dest.display().to_string() });
        Ok(json_response(&result))
    }

    /// Move or rename a file or directory.
    #[tool(description = "Move or rename a file or directory.")]
    fn move_rename(
        &self,
        Parameters(params): Parameters<CopyMoveParams>,
    ) -> Result<CallToolResult, McpError> {
        // Resolve the link itself: `rename` moves a link without following it,
        // so canonicalizing here would rename the *target* out from under the
        // link — and the fallback below would delete that target instead.
        let source = self.safe_path_nofollow(&params.source)?;
        let dest = self.safe_path_create(&params.destination)?;

        // `symlink_metadata`, not `exists()`: a dangling link is still movable.
        if fs::symlink_metadata(&source).is_err() {
            return Err(McpError::invalid_params(
                format!("source not found: {}", source.display()),
                None,
            ));
        }

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        // Try atomic rename first, fallback to copy+delete for cross-filesystem
        // moves. Both halves go through the link-preserving helpers: `is_dir()`
        // here would follow a link and hand `remove_dir_all` the target tree.
        if fs::rename(&source, &dest).is_err() {
            copy_entry(&source, &dest)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            // The copy landed, so the source goes away whole — recursively even
            // when the caller never asked, because a half-removed tree left
            // behind would be worse than either outcome.
            fsops::remove_entry(&source, true)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        let result =
            json!({ "moved": source.display().to_string(), "to": dest.display().to_string() });
        Ok(json_response(&result))
    }

    /// Delete a file or directory (recursively for directories).
    #[tool(description = "Delete a file or directory (recursively for directories).")]
    fn delete(
        &self,
        Parameters(params): Parameters<DeleteParams>,
    ) -> Result<CallToolResult, McpError> {
        // Resolve the link itself, never its target: deleting a link must not
        // destroy what it points at, and a dangling link stays deletable.
        let path = self.safe_path_nofollow(&params.path)?;

        if fs::symlink_metadata(&path).is_err() {
            return Err(McpError::invalid_params(
                format!("not found: {}", path.display()),
                None,
            ));
        }

        fsops::remove_entry(&path, params.recursive)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({ "deleted": path.display().to_string() });
        Ok(json_response(&result))
    }

    // ═══════════════════════════════════════════════════════════════
    // LOCAL FILESYSTEM — Search & Navigation
    // ═══════════════════════════════════════════════════════════════

    /// Search for files/directories matching a pattern within a directory tree.
    #[tool(
        description = "Search for files/directories matching a name pattern within a directory tree."
    )]
    fn search(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_path(&params.path)?;
        let pattern_lower = params.pattern.to_lowercase();
        let mut results = Vec::new();

        search_recursive(
            &path,
            &pattern_lower,
            params.files_only,
            params.dirs_only,
            &mut results,
            params.max_results,
        );

        let result = json!({
            "pattern": params.pattern,
            "root": path.display().to_string(),
            "count": results.len(),
            "results": results,
        });
        Ok(json_response(&result))
    }

    /// Search file contents for a text pattern (like grep).
    #[tool(
        description = "Search file contents for a text pattern (case-insensitive grep). Returns matching lines with file paths and line numbers."
    )]
    fn grep(&self, Parameters(params): Parameters<GrepParams>) -> Result<CallToolResult, McpError> {
        content_search::grep(self, &params)
    }

    /// Show directory tree structure up to a given depth.
    #[tool(description = "Show directory tree structure up to a given depth.")]
    fn tree(&self, Parameters(params): Parameters<TreeParams>) -> Result<CallToolResult, McpError> {
        let path = self.safe_dir(&params.path)?;

        let tree = render_directory_tree(&path, params.depth);

        let result = json!({
            "path": path.display().to_string(),
            "depth": params.depth,
            "tree": tree,
        });
        Ok(json_response(&result))
    }

    // ═══════════════════════════════════════════════════════════════
    // PATTERN MATCHING — Glob, Find, Replace, Delete by Pattern
    // ═══════════════════════════════════════════════════════════════

    /// Find files matching a glob pattern (e.g. "*.rs", "**/*.txt").
    #[tool(
        description = "Find files matching a glob pattern, relative to the search root (e.g. '*.rs', '**/*.txt', 'src/**/*.json'). '**' spans directories, '*' and '?' stay within one path component."
    )]
    fn glob(&self, Parameters(params): Parameters<GlobParams>) -> Result<CallToolResult, McpError> {
        pattern_search::glob(self, &params)
    }

    /// Advanced find with filters: name pattern, file type, size range, depth, modification time.
    #[tool(
        description = "Advanced file finder with filters: name glob, type (file/dir/symlink), size range, max depth, modification time."
    )]
    fn find(&self, Parameters(params): Parameters<FindParams>) -> Result<CallToolResult, McpError> {
        pattern_search::find(self, &params)
    }

    /// Find and replace text across files in a directory tree.
    #[tool(
        description = "Find and replace text in files across a directory tree. Supports extension and name filters. Use dry_run=true to preview."
    )]
    fn replace(
        &self,
        Parameters(params): Parameters<ReplaceParams>,
    ) -> Result<CallToolResult, McpError> {
        replace::replace(self, &params)
    }

    /// Delete files matching a glob pattern (e.g. "*.tmp", "*.bak").
    #[tool(
        description = "Delete files matching a glob pattern (e.g. '*.tmp', '*.bak'). Use dry_run=true to preview. Searches recursively by default."
    )]
    fn delete_by_pattern(
        &self,
        Parameters(params): Parameters<DeleteByPatternParams>,
    ) -> Result<CallToolResult, McpError> {
        cleanup::delete_by_pattern(self, &params)
    }

    // ═══════════════════════════════════════════════════════════════
    // LOCAL FILESYSTEM — System Info & Utilities
    // ═══════════════════════════════════════════════════════════════

    /// List available drives/mount points and S3 profiles.
    #[tool(description = "List available drives, mount points, and S3 profiles.")]
    fn list_drives() -> CallToolResult {
        directory_tools::list_drives()
    }

    /// Calculate total size of a file or directory tree.
    #[tool(description = "Calculate total size of a file or directory tree.")]
    fn disk_usage(
        &self,
        Parameters(params): Parameters<PathParam>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_path(&params.path)?;

        if !path.exists() {
            return Err(McpError::invalid_params(
                format!("not found: {}", path.display()),
                None,
            ));
        }

        let size = directory_size(&path);
        let result = json!({
            "path": path.display().to_string(),
            "size": size,
            "size_human": format_size(size),
        });
        Ok(json_response(&result))
    }

    /// Generate a unique filename that doesn't conflict with existing files.
    #[tool(
        description = "Generate a unique filename avoiding collisions. If 'report.txt' exists, returns 'report_1.txt', etc."
    )]
    fn unique_name(
        &self,
        Parameters(params): Parameters<UniqueNameParams>,
    ) -> Result<CallToolResult, McpError> {
        let dir = self.safe_path(&params.directory)?;
        if !dir.is_dir() {
            return Err(McpError::invalid_params(
                format!("not a directory: {}", dir.display()),
                None,
            ));
        }

        // `invalid_params`, not `internal_error`: both failures — a name that is
        // not a plain file name, and a name whose counters are all taken — are
        // properties of the caller's argument, not of the server.
        let dest = unique_destination(&dir, &params.filename)
            .map_err(|e| McpError::invalid_params(e, None))?;

        let result = json!({
            "directory": dir.display().to_string(),
            "requested": params.filename,
            "unique_path": dest.display().to_string(),
            "unique_name": dest.file_name().unwrap_or_default().to_string_lossy(),
        });
        Ok(json_response(&result))
    }

    /// Compute SHA-256 checksum of a file.
    #[tool(description = "Compute SHA-256 checksum of a file.")]
    fn file_checksum(
        &self,
        Parameters(params): Parameters<PathParam>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_file(&params.path)?;

        let hash =
            checksum_file(&path).map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let meta =
            fs::metadata(&path).map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({
            "path": path.display().to_string(),
            "sha256": hash,
            "size": meta.len(),
            "size_human": format_size(meta.len()),
        });
        Ok(json_response(&result))
    }

    /// Get human-readable size format for a byte count.
    #[tool(description = "Convert a byte count to human-readable format (e.g. 1536 → '1.50 KB').")]
    fn format_size_tool(Parameters(params): Parameters<FormatSizeParams>) -> CallToolResult {
        let result = json!({
            "bytes": params.bytes,
            "formatted": format_size(params.bytes),
        });
        json_response(&result)
    }

    // ═══════════════════════════════════════════════════════════════
    // BATCH OPERATIONS
    // ═══════════════════════════════════════════════════════════════

    /// Copy multiple files/directories to a destination directory.
    #[tool(description = "Copy multiple files/directories to a destination directory.")]
    fn batch_copy(
        &self,
        Parameters(params): Parameters<BatchParams>,
    ) -> Result<CallToolResult, McpError> {
        let dest_str = params
            .destination
            .as_deref()
            .ok_or_else(|| McpError::invalid_params("destination is required", None))?;
        let dest = self.safe_path_create(dest_str)?;
        fs::create_dir_all(&dest).map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut copied = Vec::new();
        let mut errors = Vec::new();

        // Exactly the pair `copy` uses: resolve without following the final
        // link, then let `copy_entry` reproduce whatever the source really is.
        // `sanitize_path` would canonicalize the link away and copy in the
        // contents of the file it points at.
        let copy_one = |src_path: &str| -> Result<String, String> {
            let source = paths::sanitize_path_nofollow(&self.root, src_path)?;
            let name = entry_name(&source);
            let target = join_child(&dest, &name)?;
            copy_entry(&source, &target).map_err(|e| e.to_string())?;
            Ok(name)
        };

        for src_path in &params.paths {
            match copy_one(src_path) {
                Ok(name) => copied.push(name),
                Err(error) => errors.push(json!({"path": src_path, "error": error})),
            }
        }

        let result = json!({
            "destination": dest.display().to_string(),
            "copied": copied,
            "errors": errors,
            "total_copied": copied.len(),
            "total_errors": errors.len(),
        });
        Ok(json_response(&result))
    }

    /// Move multiple files/directories to a destination directory.
    #[tool(description = "Move multiple files/directories to a destination directory.")]
    fn batch_move(
        &self,
        Parameters(params): Parameters<BatchParams>,
    ) -> Result<CallToolResult, McpError> {
        let dest_str = params
            .destination
            .as_deref()
            .ok_or_else(|| McpError::invalid_params("destination is required", None))?;
        let dest = self.safe_path_create(dest_str)?;
        fs::create_dir_all(&dest).map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut moved = Vec::new();
        let mut errors = Vec::new();

        // Exactly the pair `move_rename` uses. `sanitize_path` would resolve a
        // link to its target, and `rename` would then move the real file out
        // from under the link — or the fallback would delete the target tree.

        for src_path in &params.paths {
            match move_batch_entry(&self.root, &dest, src_path) {
                Ok(name) => moved.push(name),
                Err(error) => errors.push(json!({"path": src_path, "error": error})),
            }
        }

        let result = json!({
            "destination": dest.display().to_string(),
            "moved": moved,
            "errors": errors,
            "total_moved": moved.len(),
            "total_errors": errors.len(),
        });
        Ok(json_response(&result))
    }

    /// Delete multiple files/directories.
    #[tool(description = "Delete multiple files/directories at once.")]
    fn batch_delete(&self, Parameters(params): Parameters<BatchParams>) -> CallToolResult {
        let mut deleted = Vec::new();
        let mut errors = Vec::new();

        // Exactly the pair `delete` uses: deleting a link must not destroy what
        // it points at. `sanitize_path` hands back the target, and an
        // `is_dir()` branch follows the link a second time, so a link to a
        // directory used to be answered with `remove_dir_all` on the target.
        let delete_one = |src_path: &str| -> Result<String, String> {
            let path = paths::sanitize_path_nofollow(&self.root, src_path)?;
            fsops::remove_entry(&path, params.recursive).map_err(|e| e.to_string())?;
            Ok(path.display().to_string())
        };

        for src_path in &params.paths {
            match delete_one(src_path) {
                Ok(path) => deleted.push(path),
                Err(error) => errors.push(json!({"path": src_path, "error": error})),
            }
        }

        let result = json!({
            "deleted": deleted,
            "errors": errors,
            "total_deleted": deleted.len(),
            "total_errors": errors.len(),
        });
        json_response(&result)
    }

    // ═══════════════════════════════════════════════════════════════
    // SYMLINKS & PERMISSIONS
    // ═══════════════════════════════════════════════════════════════

    /// Create a symbolic link.
    #[tool(description = "Create a symbolic link pointing to a target path.")]
    fn create_symlink(
        &self,
        Parameters(params): Parameters<SymlinkParams>,
    ) -> Result<CallToolResult, McpError> {
        let target = self.safe_path(&params.target)?;
        let link = self.safe_path_create(&params.link)?;

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target, &link)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        #[cfg(windows)]
        {
            if target.is_dir() {
                std::os::windows::fs::symlink_dir(&target, &link)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            } else {
                std::os::windows::fs::symlink_file(&target, &link)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            }
        }

        let result = json!({
            "link": link.display().to_string(),
            "target": target.display().to_string(),
        });
        Ok(json_response(&result))
    }

    /// Read the target of a symbolic link.
    #[tool(description = "Read the target path of a symbolic link.")]
    fn read_symlink(
        &self,
        Parameters(params): Parameters<PathParam>,
    ) -> Result<CallToolResult, McpError> {
        // Only the parent is canonicalized — resolving the link would return
        // its target instead of the link.
        let path = self.safe_path_nofollow(&params.path)?;

        let target =
            fs::read_link(&path).map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({
            "link": path.display().to_string(),
            "target": target.display().to_string(),
            "target_exists": target.exists(),
        });
        Ok(json_response(&result))
    }

    /// Get or set file permissions.
    #[tool(
        description = "Get or set file permissions. Provide 'mode' (Unix octal like '755') or 'readonly' (bool) to set."
    )]
    fn permissions(
        &self,
        Parameters(params): Parameters<PermissionsParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_path(&params.path)?;

        // `readonly` runs first so that a caller supplying both gets exactly
        // the `mode` they asked for instead of a mode widened afterwards.
        if let Some(readonly) = params.readonly {
            fsops::apply_readonly(&path, readonly)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        #[cfg(unix)]
        if let Some(mode_str) = &params.mode {
            use std::os::unix::fs::PermissionsExt;
            let mode = u32::from_str_radix(mode_str, 8)
                .map_err(|e| McpError::invalid_params(format!("invalid octal mode: {e}"), None))?;
            if mode > 0o7777 {
                return Err(McpError::invalid_params(
                    format!("mode {mode_str} exceeds maximum 7777 octal"),
                    None,
                ));
            }
            fs::set_permissions(&path, fs::Permissions::from_mode(mode))
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }
        #[cfg(not(unix))]
        // `mode` remains in the cross-platform tool schema because MCP clients
        // can connect to either host. It is explicitly Unix-only and retains
        // its existing no-op behaviour elsewhere.
        let _ignored_unix_mode = params.mode.as_deref();

        let result = permissions_report(&path)?;

        Ok(json_response(&result))
    }

    // ═══════════════════════════════════════════════════════════════
    // BOOKMARKS
    // ═══════════════════════════════════════════════════════════════

    /// Save a directory path as a named bookmark.
    #[tool(description = "Save a directory path as a named bookmark for quick navigation.")]
    fn set_bookmark(
        &self,
        Parameters(params): Parameters<BookmarkParams>,
    ) -> Result<CallToolResult, McpError> {
        let path_str = params
            .path
            .as_deref()
            .ok_or_else(|| McpError::invalid_params("path is required for set_bookmark", None))?;
        let path = self.safe_path(path_str)?;

        let mut bookmarks = load_bookmarks();
        bookmarks.insert(params.name.clone(), json!(path.display().to_string()));
        save_bookmarks(&bookmarks).map_err(|e| McpError::internal_error(e, None))?;

        let result = json!({
            "bookmark": params.name,
            "path": path.display().to_string(),
        });
        Ok(json_response(&result))
    }

    /// Get the path for a named bookmark.
    #[tool(description = "Get the saved path for a named bookmark.")]
    fn get_bookmark(
        Parameters(params): Parameters<BookmarkParams>,
    ) -> Result<CallToolResult, McpError> {
        let bookmarks = load_bookmarks();
        match bookmarks.get(&params.name) {
            Some(path) => {
                let result = json!({
                    "bookmark": params.name,
                    "path": path,
                    "exists": Path::new(path.as_str().unwrap_or("")).exists(),
                });
                Ok(json_response(&result))
            }
            None => Err(McpError::invalid_params(
                format!("bookmark not found: {}", params.name),
                None,
            )),
        }
    }

    /// List all saved bookmarks.
    #[tool(description = "List all saved bookmarks with their paths.")]
    fn list_bookmarks() -> CallToolResult {
        let bookmarks = load_bookmarks();
        let entries: Vec<serde_json::Value> = bookmarks
            .iter()
            .map(|(name, path)| {
                json!({
                    "name": name,
                    "path": path,
                    "exists": Path::new(path.as_str().unwrap_or("")).exists(),
                })
            })
            .collect();

        let result = json!({
            "count": entries.len(),
            "bookmarks": entries,
        });
        json_response(&result)
    }

    /// Delete a named bookmark.
    #[tool(description = "Delete a named bookmark.")]
    fn delete_bookmark(
        Parameters(params): Parameters<BookmarkParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut bookmarks = load_bookmarks();
        if bookmarks.remove(&params.name).is_none() {
            return Err(McpError::invalid_params(
                format!("bookmark not found: {}", params.name),
                None,
            ));
        }
        save_bookmarks(&bookmarks).map_err(|e| McpError::internal_error(e, None))?;

        let result = json!({ "deleted_bookmark": params.name });
        Ok(json_response(&result))
    }

    // NOTE: there is deliberately no "open editor" tool. This server speaks
    // JSON-RPC over fd 0/1, so any child process it spawns inherits the
    // protocol streams — an interactive editor writes terminal escapes into
    // the response frames, and running a caller-named binary is arbitrary
    // code execution from a tool argument.

    // ═══════════════════════════════════════════════════════════════
    // SYSTEM INFO
    // ═══════════════════════════════════════════════════════════════

    /// Get system information: hostname, OS, architecture, home directory.
    #[tool(
        description = "Get system information: hostname, OS, architecture, home directory, current directory."
    )]
    fn system_info() -> CallToolResult {
        let mut info = json!({
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "family": std::env::consts::FAMILY,
        });

        if let Ok(cwd) = std::env::current_dir() {
            info["current_directory"] = json!(cwd.display().to_string());
        }
        if let Some(home) = dirs_home() {
            info["home_directory"] = json!(home.display().to_string());
        }

        // Hostname
        #[cfg(unix)]
        {
            if let Ok(hostname) = fs::read_to_string("/etc/hostname") {
                info["hostname"] = json!(hostname.trim());
            }
        }

        // Disk space for root/home
        #[cfg(unix)]
        if let Some(output) = std::process::Command::new("df")
            .args(["-h", "/"])
            .output()
            .ok()
            .filter(|output| output.status.success())
        {
            let text = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = text.lines().nth(1) {
                info["root_disk"] = json!(line.trim());
            }
        }

        json_response(&info)
    }

    // ═══════════════════════════════════════════════════════════════
    // ANALYSIS
    // ═══════════════════════════════════════════════════════════════

    /// Where a directory's space has gone, in one call.
    #[tool(
        description = "Analyse a directory: total size, size of each immediate subdirectory, totals per file kind (source, document, image, audio, video, archive, program, data), and the largest files. Answers 'what is taking up the space' without walking the tree by hand."
    )]
    fn analyse_directory(
        &self,
        Parameters(params): Parameters<AnalyseParams>,
    ) -> Result<CallToolResult, McpError> {
        reports::analyse_directory(self, &params)
    }

    /// Files with identical content, grouped.
    #[tool(
        description = "Find files with identical content under a directory, grouped by checksum. Only files of the same size are hashed, so a tree of unique files costs almost nothing."
    )]
    fn find_duplicates(
        &self,
        Parameters(params): Parameters<DuplicatesParams>,
    ) -> Result<CallToolResult, McpError> {
        reports::find_duplicates(self, &params)
    }

    /// The end of a file, without reading the front of it.
    #[tool(
        description = "Read the last N lines of a file. Seeks from the end rather than reading the whole file, so it is usable on a log of any size."
    )]
    fn tail_file(
        &self,
        Parameters(params): Parameters<TailParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_file(&params.path)?;
        let wanted = params.lines.unwrap_or(20);
        let (lines, total_bytes) =
            tail_lines(&path, wanted).map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({
            "path": path.display().to_string(),
            "file_size": total_bytes,
            "lines_returned": lines.len(),
            "content": lines.join("\n"),
        });
        Ok(json_response(&result))
    }

    /// What two directories disagree about.
    #[tool(
        description = "Compare two directories: which files are only on the left, only on the right, and which exist in both but differ in size. Compares names and sizes across the trees, not content."
    )]
    fn compare_directories(
        &self,
        Parameters(params): Parameters<CompareParams>,
    ) -> Result<CallToolResult, McpError> {
        let left_root = self.safe_dir(&params.left)?;
        let right_root = self.safe_dir(&params.right)?;

        let left = directory_sizes(&left_root);
        let right = directory_sizes(&right_root);

        let only_left: Vec<&String> = left
            .keys()
            .filter(|key| !right.contains_key(*key))
            .collect();
        let only_right: Vec<&String> = right
            .keys()
            .filter(|key| !left.contains_key(*key))
            .collect();
        let differing: Vec<serde_json::Value> = left
            .iter()
            .filter_map(|(name, size)| {
                let other = right.get(name)?;
                (other != size)
                    .then(|| json!({ "path": name, "left_size": size, "right_size": other }))
            })
            .collect();

        let result = json!({
            "left": left_root.display().to_string(),
            "right": right_root.display().to_string(),
            "identical": only_left.is_empty() && only_right.is_empty() && differing.is_empty(),
            "only_in_left": only_left,
            "only_in_right": only_right,
            "differing_size": differing,
        });
        Ok(json_response(&result))
    }

    /// What changed lately.
    #[tool(
        description = "List files under a directory modified within the last N days, newest first. Answers 'what have I been working on' and 'what did that build touch'."
    )]
    fn recent_files(
        &self,
        Parameters(params): Parameters<RecentParams>,
    ) -> Result<CallToolResult, McpError> {
        let root = self.safe_dir(&params.path)?;
        let days = params.days.unwrap_or(7).min(MAX_RECENT_DAYS);
        let cutoff = SystemTime::now()
            .checked_sub(Duration::from_secs(days.saturating_mul(86_400)))
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let mut files = Vec::new();
        collect_files(&root, &root, &mut files, WALK_BUDGET);

        let recent = recent_entries(&files, cutoff, params.limit.unwrap_or(50).min(10_000));

        let result = json!({
            "path": root.display().to_string(),
            "days": days,
            "count": recent.len(),
            "files": recent
                .iter()
                .map(|(modified, file)| json!({
                    "path": file.relative,
                    "size": file.size,
                    "size_human": format_size(file.size),
                    "modified_unix": modified
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .map_or(0, |since| since.as_secs()),
                }))
                .collect::<Vec<_>>(),
        });
        Ok(json_response(&result))
    }

    // ═══════════════════════════════════════════════════════════════
    // READING PARTS OF A FILE
    // ═══════════════════════════════════════════════════════════════

    /// The start of a file, without reading the rest of it.
    #[tool(
        description = "Read the first N lines of a file. Stops reading once it has them, so it is usable on a file of any size; refuses cleanly if finding them takes more than 10 MB."
    )]
    fn head_file(
        &self,
        Parameters(params): Parameters<HeadParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_file(&params.path)?;
        let wanted = params.lines.unwrap_or(20);
        let (lines, reached_end) = read_line_range(&path, 1, wanted)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({
            "path": path.display().to_string(),
            "lines_returned": lines.len(),
            "reached_end": reached_end,
            "content": lines.join("\n"),
        });
        Ok(json_response(&result))
    }

    /// A window of lines out of the middle of a file.
    #[tool(
        description = "Read a range of lines from a file, 1-based and inclusive. Reads only as far as 'to', so a window near the front of a huge file costs a window; refuses cleanly if the read passes 10 MB."
    )]
    fn read_lines(
        &self,
        Parameters(params): Parameters<LineRangeParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_file(&params.path)?;
        let from = params.from.unwrap_or(1).max(1);
        let to = params.to.unwrap_or(usize::MAX);
        if to < from {
            return Err(McpError::invalid_params(
                format!("'to' ({to}) is before 'from' ({from})"),
                None,
            ));
        }
        // `to - from` cannot overflow after the guard above.
        let (lines, reached_end) = read_line_range(&path, from, to - from + 1)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({
            "path": path.display().to_string(),
            "from": from,
            "lines_returned": lines.len(),
            "reached_end": reached_end,
            "content": lines.join("\n"),
        });
        Ok(json_response(&result))
    }

    /// How a text file is punctuated, which is what an editor has to preserve.
    #[tool(
        description = "Report a text file's line endings (lf/crlf/cr/mixed), whether it ends with a newline, its byte-order mark, and whether it looks binary. Endings are judged from the first 256 KB (see bytes_examined); the final newline is read from the true end of the file. Read before editing a file you did not write."
    )]
    fn text_format(
        &self,
        Parameters(params): Parameters<PathParam>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_file(&params.path)?;
        // One byte past the cap decides a boundary CR: followed by LF it is
        // half of a CRLF pair (counting the orphan would flip a clean CRLF
        // file to "mixed"); followed by anything else it is a genuine lone CR
        // that must stay in the count.
        let mut bytes = read_capped(&path, MAX_FORMAT_BYTES + 1)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let sampled = bytes.len() > MAX_FORMAT_BYTES;
        if sampled {
            let past_cap = bytes.pop();
            if bytes.last() == Some(&b'\r') && past_cap == Some(b'\n') {
                bytes.pop();
            }
        }
        let mut format = TextFormat::of(&bytes);
        // The final newline is a property of the file's last byte, which the
        // sample never reaches on a file larger than the cap.
        if sampled {
            format.final_newline = last_byte(&path)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
                == Some(b'\n');
        }

        let result = json!({
            "path": path.display().to_string(),
            "line_endings": format.endings,
            "crlf_count": format.crlf,
            "lf_count": format.lf,
            "cr_count": format.cr,
            "final_newline": format.final_newline,
            "byte_order_mark": format.bom,
            "binary": format.binary,
            "bytes_examined": bytes.len(),
        });
        Ok(json_response(&result))
    }

    /// `wc`, for a caller that wants the shape of a file before opening it.
    #[tool(
        description = "Count a text file's lines, words, characters and bytes, and report its longest line. Streams the file rather than loading it."
    )]
    fn file_stats(
        &self,
        Parameters(params): Parameters<PathParam>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_file(&params.path)?;
        let counts = count_file_text(&path)?;
        let on_disk = fs::metadata(&path)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .len();

        let result = json!({
            "path": path.display().to_string(),
            "lines": counts.lines,
            "words": counts.words,
            "characters": counts.chars,
            // The streamed count assumes a newline after every line; the file's
            // own length is the truth, and the two differ by one on a file that
            // does not end with a newline.
            "bytes": on_disk,
            "bytes_counted": counts.bytes,
            "longest_line_chars": counts.longest,
            "longest_line_number": counts.longest_at,
        });
        Ok(json_response(&result))
    }

    // ═══════════════════════════════════════════════════════════════
    // COMPARING AND RANKING
    // ═══════════════════════════════════════════════════════════════

    /// Whether two files are the same, and where they first stop being so.
    #[tool(
        description = "Compare two files byte for byte. Reports whether they are identical, their sizes and SHA-256 sums, and the offset of the first difference. Different sizes are answered without hashing."
    )]
    fn compare_files(
        &self,
        Parameters(params): Parameters<CompareFilesParams>,
    ) -> Result<CallToolResult, McpError> {
        let left = self.safe_file(&params.left)?;
        let right = self.safe_file(&params.right)?;
        let left_size = fs::metadata(&left)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .len();
        let right_size = fs::metadata(&right)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .len();

        // A size difference settles "identical" without hashing — but not
        // where the difference is. "short" and "much longer content" disagree
        // at byte 0, so the offset is read from the bytes, never guessed from
        // the sizes.
        let difference = first_difference(&left, &right)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut result = json!({
            "left": left.display().to_string(),
            "right": right.display().to_string(),
            "left_size": left_size,
            "right_size": right_size,
            "identical": difference.is_none(),
        });
        append_comparison_hashes(
            &mut result,
            &left,
            &right,
            difference,
            left_size == right_size,
        )?;
        Ok(json_response(&result))
    }

    /// What is taking up the space, which `disk_usage` totals but does not name.
    #[tool(
        description = "List the largest files under a directory, biggest first. Answers 'what is taking up the space', which disk_usage only totals."
    )]
    fn largest_files(
        &self,
        Parameters(params): Parameters<LargestParams>,
    ) -> Result<CallToolResult, McpError> {
        let root = self.safe_dir(&params.path)?;
        let limit = params.limit.unwrap_or(20);
        let min_size = params.min_size.unwrap_or(1);

        let mut files = Vec::new();
        collect_files(&root, &root, &mut files, WALK_BUDGET);
        // The same signal analyse_directory gives: a walk the budget stopped
        // early is a partial ranking, and a silent one misleads the caller.
        // Judged before the size filter, which hides how far the walk got.
        let truncated = files.len() >= WALK_BUDGET;
        files.retain(|file| file.size >= min_size);
        // Biggest first, and by path within a size so the answer is stable
        // between runs over the same tree.
        files.sort_by(|a, b| {
            b.size
                .cmp(&a.size)
                .then_with(|| a.relative.cmp(&b.relative))
        });
        let total: u64 = files.iter().map(|file| file.size).sum();
        // Both counted before the truncation, so the caller can tell a top-20
        // of twenty files from a top-20 of twenty thousand.
        let considered = files.len();
        files.truncate(limit);

        let result = json!({
            "path": root.display().to_string(),
            "files_considered": considered,
            "total_bytes": total,
            "truncated": truncated,
            "files": files
                .iter()
                .map(|file| json!({
                    "path": file.relative,
                    "size": file.size,
                    "size_human": format_size(file.size),
                }))
                .collect::<Vec<_>>(),
        });
        Ok(json_response(&result))
    }

    /// Where a command would come from, before running it.
    #[tool(
        description = "Find an executable on PATH, as `which`/`where` does. Returns every match in PATH order, so a shadowed binary is visible. On Unix only a file with an execute bit counts."
    )]
    fn which(Parameters(params): Parameters<WhichParams>) -> Result<CallToolResult, McpError> {
        if params.name.is_empty() || params.name.contains(['/', '\\']) {
            return Err(McpError::invalid_params(
                "name must be a bare executable name, with no directory part".to_string(),
                None,
            ));
        }
        let path_list = std::env::var_os("PATH").unwrap_or_default();
        let found = which_all(&params.name, &path_list);

        let result = json!({
            "name": params.name,
            "found": !found.is_empty(),
            "path": found.first().map(|p| p.display().to_string()),
            "all": found
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>(),
        });
        Ok(json_response(&result))
    }

    // ═══════════════════════════════════════════════════════════════
    // AWS S3 OPERATIONS
    // ═══════════════════════════════════════════════════════════════

    /// List AWS profiles from ~/.aws/credentials.
    #[tool(description = "List AWS profiles from ~/.aws/credentials file.")]
    fn s3_list_profiles() -> CallToolResult {
        s3_profiles::list()
    }

    /// List S3 buckets for a given AWS profile.
    #[tool(description = "List S3 buckets for a given AWS profile.")]
    async fn s3_list_buckets(
        &self,
        Parameters(params): Parameters<S3ProfileParam>,
    ) -> Result<CallToolResult, McpError> {
        let client = make_s3_client(&params.profile).await?;

        let resp = client
            .list_buckets()
            .send()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let buckets: Vec<serde_json::Value> = resp
            .buckets()
            .iter()
            .map(|b| {
                json!({
                    "name": b.name().unwrap_or(""),
                    "creation_date": b.creation_date().map(|d| d.to_string()),
                })
            })
            .collect();

        let result = json!({
            "profile": params.profile,
            "count": buckets.len(),
            "buckets": buckets,
        });
        Ok(json_response(&result))
    }

    /// List objects in an S3 bucket.
    #[tool(description = "List objects in an S3 bucket with optional prefix filter.")]
    async fn s3_list_objects(
        &self,
        Parameters(params): Parameters<S3ListObjectsParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = make_s3_client(&params.profile).await?;

        let prefix = params.prefix.as_deref().unwrap_or("");
        let mut request = client
            .list_objects_v2()
            .bucket(&params.bucket)
            .delimiter("/");

        if !prefix.is_empty() {
            request = request.prefix(prefix);
        }

        let resp = request
            .send()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut objects = Vec::new();

        // Folders (common prefixes)
        for prefix_obj in resp.common_prefixes() {
            if let Some(p) = prefix_obj.prefix() {
                objects.push(json!({
                    "key": p,
                    "type": "folder",
                    "size": 0,
                }));
            }
        }

        // Files
        for obj in resp.contents() {
            objects.push(json!({
                "key": obj.key().unwrap_or(""),
                "type": "file",
                "size": obj.size().unwrap_or(0),
                "size_human": format_size(object_size(obj.size())),
                "last_modified": obj.last_modified().map(std::string::ToString::to_string),
            }));
        }

        let result = json!({
            "profile": params.profile,
            "bucket": params.bucket,
            "prefix": prefix,
            "count": objects.len(),
            "objects": objects,
        });
        Ok(json_response(&result))
    }

    /// Download a file from S3 to a local path.
    #[tool(description = "Download a file from S3 to a local path.")]
    async fn s3_download(
        &self,
        Parameters(params): Parameters<S3TransferParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = make_s3_client(&params.profile).await?;
        let local_path = self.safe_path_create(&params.local_path)?;

        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        let resp = client
            .get_object()
            .bucket(&params.bucket)
            .key(&params.key)
            .send()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let bytes = resp
            .body
            .collect()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .into_bytes();

        fs::write(&local_path, &bytes)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({
            "bucket": params.bucket,
            "key": params.key,
            "local_path": local_path.display().to_string(),
            "size": bytes.len(),
            "size_human": format_size(bytes.len() as u64),
        });
        Ok(json_response(&result))
    }

    /// Upload a local file to S3.
    #[tool(description = "Upload a local file to S3.")]
    async fn s3_upload(
        &self,
        Parameters(params): Parameters<S3TransferParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = make_s3_client(&params.profile).await?;
        let local_path = self.safe_path(&params.local_path)?;

        let bytes =
            fs::read(&local_path).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let body = aws_sdk_s3::primitives::ByteStream::from(bytes.clone());

        client
            .put_object()
            .bucket(&params.bucket)
            .key(&params.key)
            .body(body)
            .send()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({
            "bucket": params.bucket,
            "key": params.key,
            "local_path": local_path.display().to_string(),
            "size": bytes.len(),
            "size_human": format_size(bytes.len() as u64),
        });
        Ok(json_response(&result))
    }

    /// Delete an object from S3.
    #[tool(description = "Delete an object from S3.")]
    async fn s3_delete(
        &self,
        Parameters(params): Parameters<S3DeleteParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = make_s3_client(&params.profile).await?;

        client
            .delete_object()
            .bucket(&params.bucket)
            .key(&params.key)
            .send()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({
            "bucket": params.bucket,
            "deleted": params.key,
        });
        Ok(json_response(&result))
    }

    /// Copy an object within S3 (same or different bucket).
    #[tool(description = "Copy an S3 object within or between buckets.")]
    async fn s3_copy(
        &self,
        Parameters(params): Parameters<S3CopyParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = make_s3_client(&params.profile).await?;

        let copy_source = format!("{}/{}", params.source_bucket, params.source_key);
        client
            .copy_object()
            .bucket(&params.dest_bucket)
            .key(&params.dest_key)
            .copy_source(&copy_source)
            .send()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({
            "source": format!("s3://{}/{}", params.source_bucket, params.source_key),
            "destination": format!("s3://{}/{}", params.dest_bucket, params.dest_key),
        });
        Ok(json_response(&result))
    }
}

// ─── Additional Parameter Types ─────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct FormatSizeParams {
    /// Number of bytes to format
    bytes: u64,
}

// Dispatch through the router built in `new()` — the default
// `Self::tool_router()` would rebuild it on every call and leaves the field
// unread (a `dead_code` warning that `clippy -D warnings` turns into an error).
#[tool_handler(router = self.tool_router)]
impl ServerHandler for FfmServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(
                "FunnyFileManager MCP server — comprehensive file management tools for AI assistants. \
                 Supports local filesystem operations (list, view, create, copy, move, delete, search, grep, tree), \
                 pattern matching (glob, find with filters, replace in files, delete by pattern), \
                 batch operations, bookmarks, symlinks, permissions, checksums, \
                 read-only KeePass export to JSON, YAML, or dotenv, \
                 S3 cloud storage (list profiles/buckets/objects, upload, download, delete, copy), \
                 and system information. All paths are confined to the server's allowed root."
                    .to_owned(),
            )
    }
}

// ─── Main ───────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Log to stderr only — stdout is reserved for JSON-RPC
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    // Resolved once: every tool argument is confined to this tree.
    let root = paths::resolve_root()?;
    info!(root = %root.display(), tools = TOOL_NAMES.len(), "ffm-mcp server starting");

    let service = FfmServer::new(root).serve(stdio()).await?;
    service.waiting().await?;

    info!("ffm-mcp server stopped");
    Ok(())
}

fn json_response(value: &serde_json::Value) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(
        serde_json::to_string_pretty(value).unwrap_or_default(),
    )])
}

fn read_tail_bytes(file: &mut fs::File, size: u64, wanted: usize) -> io::Result<Vec<u8>> {
    const BLOCK: usize = 8 * 1024;
    let mut buffer: Vec<u8> = Vec::new();
    let mut position = size;
    let mut newlines = 0usize;
    // One more newline than lines wanted: the last block read may begin
    // mid-line, and that partial line is not one of the answers.
    while position > 0 && newlines <= wanted && buffer.len() < MAX_TAIL_BYTES {
        let room = MAX_TAIL_BYTES - buffer.len();
        let step = u64::try_from(BLOCK.min(room))
            .unwrap_or(u64::MAX)
            .min(position);
        if step == 0 {
            break;
        }
        position -= step;
        file.seek(SeekFrom::Start(position))?;
        let mut block = vec![0u8; usize::try_from(step).unwrap_or(BLOCK)];
        file.read_exact(&mut block)?;
        newlines += block.iter().filter(|byte| **byte == b'\n').count();
        block.extend_from_slice(&buffer);
        buffer = block;
    }

    Ok(buffer)
}

fn sort_listing(entries: &mut [serde_json::Value]) {
    entries.sort_by(|a, b| {
        let a_is_dir = a["type"] == "dir";
        let b_is_dir = b["type"] == "dir";
        b_is_dir.cmp(&a_is_dir).then_with(|| {
            a["name"]
                .as_str()
                .unwrap_or("")
                .cmp(b["name"].as_str().unwrap_or(""))
        })
    });
}

fn append_file_metadata(
    info: &mut serde_json::Value,
    path: &Path,
    meta: &fs::Metadata,
    modified: Option<u64>,
) {
    if let Some(ts) = modified {
        info["modified_unix"] = json!(ts);
    }
    if let Some(ext) = path.extension() {
        info["extension"] = json!(ext.to_string_lossy());
    }

    // Unix permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        info["mode"] = json!(format!("{:o}", meta.permissions().mode()));
    }

    // Symlink target
    if meta.is_symlink() {
        if let Ok(target) = fs::read_link(path) {
            info["symlink_target"] = json!(target.display().to_string());
        }
    }
}

fn validate_vault_extension(path: &Path) -> Result<(), McpError> {
    let supported_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("kdbx") || extension.eq_ignore_ascii_case("kbdx")
        });
    if !supported_extension {
        return Err(McpError::invalid_params(
            "KeePass database path must end in .kdbx or .kbdx",
            None,
        ));
    }

    Ok(())
}

fn export_error(error: kdbx::ExportError) -> McpError {
    match error {
        kdbx::ExportError::Read(source) => {
            McpError::internal_error(format!("failed to read KeePass data: {source}"), None)
        }
        kdbx::ExportError::InvalidKeyFile => {
            McpError::invalid_params("invalid KeePass key file", None)
        }
        kdbx::ExportError::Decrypt => McpError::invalid_params(
            "could not decrypt KeePass database (wrong credentials or invalid database)",
            None,
        ),
        kdbx::ExportError::Serialize => {
            McpError::internal_error("could not serialize KeePass database", None)
        }
        kdbx::ExportError::DatabaseTooLarge(bytes) => McpError::invalid_params(
            format!(
                "KeePass database is {bytes} bytes; maximum is {} bytes",
                kdbx::MAX_DATABASE_BYTES
            ),
            None,
        ),
        kdbx::ExportError::OutputTooLarge(bytes) => McpError::invalid_params(
            format!(
                "KeePass text export is {bytes} bytes; maximum is {} bytes",
                kdbx::MAX_OUTPUT_BYTES
            ),
            None,
        ),
    }
}

fn move_batch_entry(root: &Path, dest: &Path, src_path: &str) -> Result<String, String> {
    let source = paths::sanitize_path_nofollow(root, src_path)?;
    let name = entry_name(&source);
    let target = join_child(dest, &name)?;
    if fs::rename(&source, &target).is_err() {
        copy_entry(&source, &target).map_err(|e| e.to_string())?;
        // The copy landed, so the source goes away whole.
        fsops::remove_entry(&source, true).map_err(|e| e.to_string())?;
    }
    Ok(name)
}

fn permissions_report(path: &Path) -> Result<serde_json::Value, McpError> {
    // Re-read so the response reports what is actually on disk.
    let perms = fs::metadata(path)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
        .permissions();

    #[cfg(unix)]
    let reported_mode = {
        use std::os::unix::fs::PermissionsExt;
        Some(format!("{:o}", perms.mode()))
    };
    #[cfg(not(unix))]
    let reported_mode: Option<String> = None;

    let mut result = json!({
        "path": path.display().to_string(),
        "readonly": perms.readonly(),
    });

    if let Some(mode) = reported_mode {
        result["mode"] = json!(mode);
    }

    Ok(result)
}

fn directory_sizes(root: &Path) -> BTreeMap<String, u64> {
    let mut files = Vec::new();
    collect_files(root, root, &mut files, WALK_BUDGET);
    files
        .into_iter()
        .map(|file| (file.relative, file.size))
        .collect::<BTreeMap<_, _>>()
}

fn recent_entries(
    files: &[CollectedFile],
    cutoff: SystemTime,
    limit: usize,
) -> Vec<(SystemTime, &CollectedFile)> {
    let mut recent: Vec<(SystemTime, &CollectedFile)> = files
        .iter()
        .filter_map(|file| {
            let modified = fs::metadata(&file.path).ok()?.modified().ok()?;
            (modified >= cutoff).then_some((modified, file))
        })
        .collect();
    recent.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    recent.truncate(limit);

    recent
}

struct TextCounts {
    lines: u64,
    words: u64,
    chars: u64,
    bytes: u64,
    longest: usize,
    longest_at: u64,
}

fn count_file_text(path: &Path) -> Result<TextCounts, McpError> {
    let file = fs::File::open(path).map_err(|e| McpError::internal_error(e.to_string(), None))?;
    let mut lines = 0u64;
    let mut words = 0u64;
    let mut chars = 0u64;
    let mut bytes = 0u64;
    let mut longest = 0usize;
    let mut longest_at = 0u64;

    for (number, line) in io::BufRead::split(io::BufReader::new(file), b'\n').enumerate() {
        let line = line.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = String::from_utf8_lossy(&line);
        let char_count = text.chars().count();
        lines += 1;
        words += text.split_whitespace().count() as u64;
        chars += char_count as u64;
        // `split` dropped the separator; every line but a last one without
        // a trailing newline had one.
        bytes += line.len() as u64 + 1;
        if char_count > longest {
            longest = char_count;
            longest_at = number as u64 + 1;
        }
    }
    Ok(TextCounts {
        lines,
        words,
        chars,
        bytes,
        longest,
        longest_at,
    })
}

fn append_comparison_hashes(
    result: &mut serde_json::Value,
    left: &Path,
    right: &Path,
    difference: Option<u64>,
    equal_size: bool,
) -> Result<(), McpError> {
    match difference {
        None => {
            result["sha256"] =
                json!(checksum_file(left)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?);
        }
        Some(offset) => {
            result["first_difference_at"] = json!(offset);
            if equal_size {
                result["left_sha256"] = json!(checksum_file(left)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?);
                result["right_sha256"] = json!(checksum_file(right)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?);
            }
        }
    }
    Ok(())
}
