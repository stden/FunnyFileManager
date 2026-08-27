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
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write as IoWrite};
use std::path::{Component, Path, PathBuf};
use tracing::info;

use ffm_mcp::tree_walk::{directory_size, render_directory_tree};

mod fsops;
mod globmatch;
mod paths;
// The tool list lives in its own import-free file so the sibling crate's MCP
// status screen can `include!` it instead of keeping a second copy.
mod tool_names;
pub use tool_names::TOOL_NAMES;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_tools;

// ─── Utility ────────────────────────────────────────────────────

fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;
    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{size} B")
    }
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
        let path = self.safe_path(&params.path)?;

        if !path.is_dir() {
            return Err(McpError::invalid_params(
                format!("not a directory: {}", path.display()),
                None,
            ));
        }

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

        let result = json!({
            "path": path.display().to_string(),
            "count": entries.len(),
            "entries": entries,
        });
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    /// List directory sorted by name, extension, or size (like Panel sort modes).
    #[tool(
        description = "List directory sorted by name, extension, or size. Sort modes: 'name', 'extension', 'size'."
    )]
    fn sort_directory(
        &self,
        Parameters(params): Parameters<SortDirectoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_path(&params.path)?;

        if !path.is_dir() {
            return Err(McpError::invalid_params(
                format!("not a directory: {}", path.display()),
                None,
            ));
        }

        let mut entries = Vec::new();
        for entry in
            fs::read_dir(&path).map_err(|e| McpError::internal_error(e.to_string(), None))?
        {
            let entry = entry.map_err(|e| McpError::internal_error(e.to_string(), None))?;
            let meta = entry
                .metadata()
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            let name = entry.file_name().to_string_lossy().to_string();
            let ext = Path::new(&name)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_owned();
            entries.push(json!({
                "name": name,
                "type": if meta.is_dir() { "dir" } else { "file" },
                "size": meta.len(),
                "size_human": format_size(meta.len()),
                "extension": ext,
            }));
        }

        // Dirs first, then sort within groups
        match params.sort.as_str() {
            "extension" | "ext" => {
                entries.sort_by(|a, b| {
                    let a_dir = a["type"] == "dir";
                    let b_dir = b["type"] == "dir";
                    b_dir.cmp(&a_dir).then_with(|| {
                        a["extension"]
                            .as_str()
                            .unwrap_or("")
                            .cmp(b["extension"].as_str().unwrap_or(""))
                            .then_with(|| {
                                a["name"]
                                    .as_str()
                                    .unwrap_or("")
                                    .cmp(b["name"].as_str().unwrap_or(""))
                            })
                    })
                });
            }
            "size" => {
                entries.sort_by(|a, b| {
                    let a_dir = a["type"] == "dir";
                    let b_dir = b["type"] == "dir";
                    b_dir.cmp(&a_dir).then_with(|| {
                        b["size"]
                            .as_u64()
                            .unwrap_or(0)
                            .cmp(&a["size"].as_u64().unwrap_or(0))
                    })
                });
            }
            _ => {
                // Default: sort by name
                entries.sort_by(|a, b| {
                    let a_dir = a["type"] == "dir";
                    let b_dir = b["type"] == "dir";
                    b_dir.cmp(&a_dir).then_with(|| {
                        a["name"]
                            .as_str()
                            .unwrap_or("")
                            .cmp(b["name"].as_str().unwrap_or(""))
                    })
                });
            }
        }

        let result = json!({
            "path": path.display().to_string(),
            "sort": params.sort,
            "count": entries.len(),
            "entries": entries,
        });
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
            if let Ok(target) = fs::read_link(&path) {
                info["symlink_target"] = json!(target.display().to_string());
            }
        }

        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&info).unwrap_or_default(),
        )]))
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
        const MAX_SIZE: u64 = 10 * 1024 * 1024;
        if meta.len() > MAX_SIZE {
            return Err(McpError::invalid_params(
                format!("file too large: {} (max 10 MB)", format_size(meta.len())),
                None,
            ));
        }

        let content =
            fs::read_to_string(&path).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        let offset = params.offset.unwrap_or(0) as usize;
        let limit = params.limit.map(|l| l as usize).unwrap_or(total_lines);

        let selected: Vec<&str> = lines.into_iter().skip(offset).take(limit).collect();

        let result = json!({
            "path": path.display().to_string(),
            "total_lines": total_lines,
            "offset": offset,
            "lines_returned": selected.len(),
            "content": selected.join("\n"),
        });
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    /// Search file contents for a text pattern (like grep).
    #[tool(
        description = "Search file contents for a text pattern (case-insensitive grep). Returns matching lines with file paths and line numbers."
    )]
    fn grep(&self, Parameters(params): Parameters<GrepParams>) -> Result<CallToolResult, McpError> {
        let path = self.safe_path(&params.path)?;
        let pattern_lower = params.pattern.to_lowercase();
        let mut results = Vec::new();

        fn grep_recursive(
            dir: &Path,
            pattern: &str,
            ext_filter: Option<&str>,
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
                // A link is never opened: `File::open` below would follow it out
                // of the server root (see `walkable_type`).
                let Some(file_type) = walkable_type(&entry) else {
                    continue;
                };
                let path = entry.path();

                if file_type.is_dir() {
                    grep_recursive(&path, pattern, ext_filter, results, max);
                    continue;
                }

                // Check extension filter
                if let Some(ext) = ext_filter {
                    let file_ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if !file_ext.eq_ignore_ascii_case(ext) {
                        continue;
                    }
                }

                // Skip binary/large files
                let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                if size > 10 * 1024 * 1024 {
                    continue;
                }

                if let Ok(file) = fs::File::open(&path) {
                    use std::io::BufRead;
                    let reader = io::BufReader::new(file);
                    for (line_num, line) in reader.lines().enumerate() {
                        if results.len() >= max {
                            return;
                        }
                        let line = match line {
                            Ok(l) => l,
                            Err(_) => break, // binary file or encoding error
                        };
                        if line.to_lowercase().contains(pattern) {
                            results.push(json!({
                                "file": path.display().to_string(),
                                "line": line_num + 1,
                                "content": line.trim(),
                            }));
                        }
                    }
                }
            }
        }

        grep_recursive(
            &path,
            &pattern_lower,
            params.extension.as_deref(),
            &mut results,
            params.max_results,
        );

        let result = json!({
            "pattern": params.pattern,
            "root": path.display().to_string(),
            "count": results.len(),
            "matches": results,
        });
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    /// Show directory tree structure up to a given depth.
    #[tool(description = "Show directory tree structure up to a given depth.")]
    fn tree(&self, Parameters(params): Parameters<TreeParams>) -> Result<CallToolResult, McpError> {
        let path = self.safe_path(&params.path)?;

        if !path.is_dir() {
            return Err(McpError::invalid_params(
                format!("not a directory: {}", path.display()),
                None,
            ));
        }

        let tree = render_directory_tree(&path, params.depth);

        let result = json!({
            "path": path.display().to_string(),
            "depth": params.depth,
            "tree": tree,
        });
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    // ═══════════════════════════════════════════════════════════════
    // PATTERN MATCHING — Glob, Find, Replace, Delete by Pattern
    // ═══════════════════════════════════════════════════════════════

    /// Find files matching a glob pattern (e.g. "*.rs", "**/*.txt").
    #[tool(
        description = "Find files matching a glob pattern, relative to the search root (e.g. '*.rs', '**/*.txt', 'src/**/*.json'). '**' spans directories, '*' and '?' stay within one path component."
    )]
    fn glob(&self, Parameters(params): Parameters<GlobParams>) -> Result<CallToolResult, McpError> {
        let root = self.safe_path(&params.path)?;
        if !root.is_dir() {
            return Err(McpError::invalid_params(
                format!("not a directory: {}", root.display()),
                None,
            ));
        }

        struct GlobScan<'a> {
            root: &'a Path,
            pattern: &'a str,
            /// `None` once `**` makes the depth unbounded.
            max_depth: Option<usize>,
            max_results: usize,
        }

        fn glob_recursive(
            dir: &Path,
            depth: usize,
            scan: &GlobScan<'_>,
            results: &mut Vec<serde_json::Value>,
        ) {
            if results.len() >= scan.max_results {
                return;
            }
            let entries = match fs::read_dir(dir) {
                Ok(e) => e,
                Err(_) => return,
            };
            for entry in entries.flatten() {
                if results.len() >= scan.max_results {
                    return;
                }
                // A link is never reported: `entry.metadata()` below follows it,
                // so its size would describe a file outside the server root
                // (see `walkable_type`).
                let Some(file_type) = walkable_type(&entry) else {
                    continue;
                };
                let path = entry.path();
                let is_dir = file_type.is_dir();
                let relative = relative_slash_path(scan.root, &path);

                if globmatch::matches(&relative, scan.pattern) {
                    let meta = entry.metadata().ok();
                    results.push(json!({
                        "path": path.display().to_string(),
                        "name": entry.file_name().to_string_lossy(),
                        "relative_path": relative,
                        "type": if is_dir { "dir" } else { "file" },
                        "size": meta.as_ref().map(|m| m.len()).unwrap_or(0),
                    }));
                }

                let descend = match scan.max_depth {
                    Some(max) => depth < max,
                    None => true,
                };
                if is_dir && descend {
                    glob_recursive(&path, depth + 1, scan, results);
                }
            }
        }

        let scan = GlobScan {
            root: &root,
            pattern: &params.pattern,
            max_depth: globmatch::max_depth(&params.pattern),
            max_results: params.max_results,
        };
        let mut results = Vec::new();
        glob_recursive(&root, 0, &scan, &mut results);

        let result = json!({
            "pattern": params.pattern,
            "root": root.display().to_string(),
            "count": results.len(),
            "results": results,
        });
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    /// Advanced find with filters: name pattern, file type, size range, depth, modification time.
    #[tool(
        description = "Advanced file finder with filters: name glob, type (file/dir/symlink), size range, max depth, modification time."
    )]
    fn find(&self, Parameters(params): Parameters<FindParams>) -> Result<CallToolResult, McpError> {
        let root = self.safe_path(&params.path)?;
        if !root.is_dir() {
            return Err(McpError::invalid_params(
                format!("not a directory: {}", root.display()),
                None,
            ));
        }

        let now = std::time::SystemTime::now();
        let mut results = Vec::new();

        fn find_recursive(
            dir: &Path,
            depth: usize,
            params: &FindParams,
            now: &std::time::SystemTime,
            results: &mut Vec<serde_json::Value>,
        ) {
            if results.len() >= params.max_results {
                return;
            }
            if let Some(max_d) = params.max_depth {
                if depth > max_d {
                    return;
                }
            }

            let entries = match fs::read_dir(dir) {
                Ok(e) => e,
                Err(_) => return,
            };

            for entry in entries.flatten() {
                if results.len() >= params.max_results {
                    return;
                }

                let name = entry.file_name().to_string_lossy().to_string();
                let meta = match fs::symlink_metadata(entry.path()) {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let is_dir = meta.is_dir();
                let is_symlink = meta.is_symlink();
                let file_type_str = if is_symlink {
                    "symlink"
                } else if is_dir {
                    "dir"
                } else {
                    "file"
                };

                // Apply filters
                let mut matches = true;

                // Name pattern (glob against the entry name)
                if let Some(ref pattern) = params.name {
                    matches = globmatch::matches(&name, pattern);
                }

                // Type filter
                if matches {
                    if let Some(ref ft) = params.file_type {
                        matches = ft == file_type_str;
                    }
                }

                // Size filters
                if matches {
                    if let Some(min) = params.min_size {
                        if meta.len() < min {
                            matches = false;
                        }
                    }
                    if let Some(max) = params.max_size {
                        if meta.len() > max {
                            matches = false;
                        }
                    }
                }

                // Modified time filter
                if matches {
                    if let Some(secs) = params.modified_within_secs {
                        if let Ok(modified) = meta.modified() {
                            if let Ok(elapsed) = now.duration_since(modified) {
                                if elapsed.as_secs() > secs {
                                    matches = false;
                                }
                            }
                        }
                    }
                }

                if matches {
                    results.push(json!({
                        "path": entry.path().display().to_string(),
                        "name": name,
                        "type": file_type_str,
                        "size": meta.len(),
                        "size_human": format_size(meta.len()),
                    }));
                }

                if is_dir && !is_symlink {
                    find_recursive(&entry.path(), depth + 1, params, now, results);
                }
            }
        }

        find_recursive(&root, 0, &params, &now, &mut results);

        let result = json!({
            "root": root.display().to_string(),
            "count": results.len(),
            "results": results,
        });
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    /// Find and replace text across files in a directory tree.
    #[tool(
        description = "Find and replace text in files across a directory tree. Supports extension and name filters. Use dry_run=true to preview."
    )]
    fn replace(
        &self,
        Parameters(params): Parameters<ReplaceParams>,
    ) -> Result<CallToolResult, McpError> {
        let root = self.safe_path(&params.path)?;
        if !root.is_dir() {
            return Err(McpError::invalid_params(
                format!("not a directory: {}", root.display()),
                None,
            ));
        }

        if params.find.is_empty() {
            return Err(McpError::invalid_params(
                "find pattern must not be empty",
                None,
            ));
        }

        struct ReplaceOpts<'a> {
            find: &'a str,
            replace_with: &'a str,
            ext_filter: Option<&'a str>,
            name_pattern: Option<&'a str>,
            dry_run: bool,
        }

        #[derive(Default)]
        struct ReplaceRun {
            changed: Vec<serde_json::Value>,
            errors: Vec<serde_json::Value>,
            total: usize,
        }

        fn replace_recursive(dir: &Path, opts: &ReplaceOpts<'_>, run: &mut ReplaceRun) {
            let entries = match fs::read_dir(dir) {
                Ok(e) => e,
                Err(_) => return,
            };
            for entry in entries.flatten() {
                // A link is never opened: `read_to_string` below would follow it
                // out of the server root, and the rewrite would then copy that
                // outside content back in (see `walkable_type`).
                let Some(file_type) = walkable_type(&entry) else {
                    continue;
                };
                let path = entry.path();

                if file_type.is_dir() {
                    replace_recursive(&path, opts, run);
                    continue;
                }

                let name = entry.file_name().to_string_lossy().to_string();

                // Extension filter
                if let Some(ext) = opts.ext_filter {
                    let file_ext = Path::new(&name)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");
                    if !file_ext.eq_ignore_ascii_case(ext) {
                        continue;
                    }
                }

                // Name pattern filter
                if let Some(pattern) = opts.name_pattern {
                    if !globmatch::matches(&name, pattern) {
                        continue;
                    }
                }

                // Skip binary/large files
                let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                if size > 10 * 1024 * 1024 {
                    continue;
                }

                let content = match fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                let count = content.matches(opts.find).count();
                if count == 0 {
                    continue;
                }

                if opts.dry_run {
                    run.total += count;
                    run.changed.push(json!({
                        "path": path.display().to_string(),
                        "replacements": count,
                    }));
                    continue;
                }

                // Count the replacements only once the write really landed —
                // a failed write must be reported, not reported as success.
                let new_content = content.replace(opts.find, opts.replace_with);
                match fsops::write_replaced(&path, &new_content) {
                    Ok(()) => {
                        run.total += count;
                        run.changed.push(json!({
                            "path": path.display().to_string(),
                            "replacements": count,
                        }));
                    }
                    Err(e) => run.errors.push(json!({
                        "path": path.display().to_string(),
                        "error": e.to_string(),
                    })),
                }
            }
        }

        let opts = ReplaceOpts {
            find: &params.find,
            replace_with: &params.replace,
            ext_filter: params.extension.as_deref(),
            name_pattern: params.name_pattern.as_deref(),
            dry_run: params.dry_run,
        };
        let mut run = ReplaceRun::default();
        replace_recursive(&root, &opts, &mut run);

        let result = json!({
            "root": root.display().to_string(),
            "find": params.find,
            "replace": params.replace,
            "dry_run": params.dry_run,
            "files_affected": run.changed.len(),
            "total_replacements": run.total,
            "files": run.changed,
            "errors": run.errors.len(),
            "error_details": run.errors,
        });
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    /// Delete files matching a glob pattern (e.g. "*.tmp", "*.bak").
    #[tool(
        description = "Delete files matching a glob pattern (e.g. '*.tmp', '*.bak'). Use dry_run=true to preview. Searches recursively by default."
    )]
    fn delete_by_pattern(
        &self,
        Parameters(params): Parameters<DeleteByPatternParams>,
    ) -> Result<CallToolResult, McpError> {
        let root = self.safe_path(&params.path)?;
        if !root.is_dir() {
            return Err(McpError::invalid_params(
                format!("not a directory: {}", root.display()),
                None,
            ));
        }

        let mut deleted = Vec::new();
        let mut errors = Vec::new();

        fn delete_pattern_recursive(
            dir: &Path,
            pattern: &str,
            include_dirs: bool,
            recursive: bool,
            dry_run: bool,
            deleted: &mut Vec<serde_json::Value>,
            errors: &mut Vec<serde_json::Value>,
        ) {
            let entries = match fs::read_dir(dir) {
                Ok(e) => e,
                Err(_) => return,
            };
            for entry in entries.flatten() {
                // A link is never matched: `dir_size`/`fs::metadata` below follow
                // it out of the server root, and a link that happens to match the
                // pattern is not the file the caller asked to delete (see
                // `walkable_type`).
                let Some(file_type) = walkable_type(&entry) else {
                    continue;
                };
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                if file_type.is_dir() {
                    if recursive {
                        delete_pattern_recursive(
                            &path,
                            pattern,
                            include_dirs,
                            recursive,
                            dry_run,
                            deleted,
                            errors,
                        );
                    }
                    if include_dirs && globmatch::matches(&name, pattern) {
                        let size = directory_size(&path);
                        if dry_run {
                            deleted.push(json!({
                                "path": path.display().to_string(),
                                "type": "dir",
                                "size": size,
                                "would_delete": true,
                            }));
                        } else {
                            match fs::remove_dir_all(&path) {
                                Ok(()) => deleted.push(json!({
                                    "path": path.display().to_string(),
                                    "type": "dir",
                                    "size": size,
                                })),
                                Err(e) => errors.push(json!({
                                    "path": path.display().to_string(),
                                    "error": e.to_string(),
                                })),
                            }
                        }
                    }
                } else if globmatch::matches(&name, pattern) {
                    let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    if dry_run {
                        deleted.push(json!({
                            "path": path.display().to_string(),
                            "type": "file",
                            "size": size,
                            "would_delete": true,
                        }));
                    } else {
                        match fs::remove_file(&path) {
                            Ok(()) => deleted.push(json!({
                                "path": path.display().to_string(),
                                "type": "file",
                                "size": size,
                            })),
                            Err(e) => errors.push(json!({
                                "path": path.display().to_string(),
                                "error": e.to_string(),
                            })),
                        }
                    }
                }
            }
        }

        delete_pattern_recursive(
            &root,
            &params.pattern,
            params.include_dirs,
            params.recursive,
            params.dry_run,
            &mut deleted,
            &mut errors,
        );

        let total_size: u64 = deleted.iter().filter_map(|d| d["size"].as_u64()).sum();

        let result = json!({
            "root": root.display().to_string(),
            "pattern": params.pattern,
            "dry_run": params.dry_run,
            "deleted_count": deleted.len(),
            "total_size": total_size,
            "total_size_human": format_size(total_size),
            "errors": errors.len(),
            "deleted": deleted,
            "error_details": errors,
        });
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    // ═══════════════════════════════════════════════════════════════
    // LOCAL FILESYSTEM — System Info & Utilities
    // ═══════════════════════════════════════════════════════════════

    /// List available drives/mount points and S3 profiles.
    #[tool(description = "List available drives, mount points, and S3 profiles.")]
    fn list_drives(&self) -> Result<CallToolResult, McpError> {
        let mut drives = Vec::new();

        #[cfg(target_os = "windows")]
        {
            for letter in b'A'..=b'Z' {
                let drive = format!("{}:\\", letter as char);
                if Path::new(&drive).exists() {
                    drives.push(json!({ "type": "local", "path": drive }));
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            drives.push(json!({ "type": "local", "path": "/" }));
            for mount in &["/home", "/tmp", "/var", "/opt", "/mnt", "/media"] {
                if Path::new(mount).is_dir() {
                    drives.push(json!({ "type": "mount", "path": mount }));
                }
            }
        }

        // AWS S3 profiles from ~/.aws/credentials
        if let Some(home) = dirs_home() {
            let creds_path = home.join(".aws").join("credentials");
            if creds_path.exists() {
                if let Ok(content) = fs::read_to_string(&creds_path) {
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if let Some(name) =
                            trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']'))
                        {
                            drives.push(json!({ "type": "s3", "profile": name }));
                        }
                    }
                }
            }
        }

        let result = json!({ "drives": drives });
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    /// Compute SHA-256 checksum of a file.
    #[tool(description = "Compute SHA-256 checksum of a file.")]
    fn file_checksum(
        &self,
        Parameters(params): Parameters<PathParam>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.safe_path(&params.path)?;

        if !path.is_file() {
            return Err(McpError::invalid_params(
                format!("not a file: {}", path.display()),
                None,
            ));
        }

        let mut file =
            fs::File::open(&path).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];
        loop {
            let n = file
                .read(&mut buffer)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        // sha2 0.11's `finalize()` returns an `Array` that no longer implements
        // `LowerHex`, so build the hex string from the raw bytes ourselves.
        let hash: String = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();

        let meta =
            fs::metadata(&path).map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({
            "path": path.display().to_string(),
            "sha256": hash,
            "size": meta.len(),
            "size_human": format_size(meta.len()),
        });
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    /// Get human-readable size format for a byte count.
    #[tool(description = "Convert a byte count to human-readable format (e.g. 1536 → '1.50 KB').")]
    fn format_size_tool(
        &self,
        Parameters(params): Parameters<FormatSizeParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = json!({
            "bytes": params.bytes,
            "formatted": format_size(params.bytes),
        });
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
        let move_one = |src_path: &str| -> Result<String, String> {
            let source = paths::sanitize_path_nofollow(&self.root, src_path)?;
            let name = entry_name(&source);
            let target = join_child(&dest, &name)?;
            if fs::rename(&source, &target).is_err() {
                copy_entry(&source, &target).map_err(|e| e.to_string())?;
                // The copy landed, so the source goes away whole.
                fsops::remove_entry(&source, true).map_err(|e| e.to_string())?;
            }
            Ok(name)
        };

        for src_path in &params.paths {
            match move_one(src_path) {
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    /// Delete multiple files/directories.
    #[tool(description = "Delete multiple files/directories at once.")]
    fn batch_delete(
        &self,
        Parameters(params): Parameters<BatchParams>,
    ) -> Result<CallToolResult, McpError> {
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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

        // Re-read so the response reports what is actually on disk.
        let perms = fs::metadata(&path)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .permissions();

        let mut result = json!({
            "path": path.display().to_string(),
            "readonly": perms.readonly(),
        });

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            result["mode"] = json!(format!("{:o}", perms.mode()));
        }

        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    /// Get the path for a named bookmark.
    #[tool(description = "Get the saved path for a named bookmark.")]
    fn get_bookmark(
        &self,
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
                Ok(CallToolResult::success(vec![ContentBlock::text(
                    serde_json::to_string_pretty(&result).unwrap_or_default(),
                )]))
            }
            None => Err(McpError::invalid_params(
                format!("bookmark not found: {}", params.name),
                None,
            )),
        }
    }

    /// List all saved bookmarks.
    #[tool(description = "List all saved bookmarks with their paths.")]
    fn list_bookmarks(&self) -> Result<CallToolResult, McpError> {
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    /// Delete a named bookmark.
    #[tool(description = "Delete a named bookmark.")]
    fn delete_bookmark(
        &self,
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
    fn system_info(&self) -> Result<CallToolResult, McpError> {
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
        {
            if let Ok(output) = std::process::Command::new("df").args(["-h", "/"]).output() {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout);
                    if let Some(line) = text.lines().nth(1) {
                        info["root_disk"] = json!(line.trim());
                    }
                }
            }
        }

        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&info).unwrap_or_default(),
        )]))
    }

    // ═══════════════════════════════════════════════════════════════
    // AWS S3 OPERATIONS
    // ═══════════════════════════════════════════════════════════════

    /// List AWS profiles from ~/.aws/credentials.
    #[tool(description = "List AWS profiles from ~/.aws/credentials file.")]
    fn s3_list_profiles(&self) -> Result<CallToolResult, McpError> {
        let mut profiles = Vec::new();

        if let Some(home) = dirs_home() {
            let creds_path = home.join(".aws").join("credentials");
            if creds_path.exists() {
                if let Ok(content) = fs::read_to_string(&creds_path) {
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if let Some(name) =
                            trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']'))
                        {
                            profiles.push(json!({ "name": name }));
                        }
                    }
                }
            }

            // Also check config file for region info
            let config_path = home.join(".aws").join("config");
            if config_path.exists() {
                if let Ok(content) = fs::read_to_string(&config_path) {
                    let mut current_profile = String::new();
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if let Some(name) =
                            trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']'))
                        {
                            current_profile =
                                name.strip_prefix("profile ").unwrap_or(name).to_owned();
                        } else if let Some(region) = trimmed.strip_prefix("region") {
                            let region = region.trim_start_matches(['=', ' ']);
                            for p in &mut profiles {
                                if p["name"].as_str() == Some(&current_profile) {
                                    p["region"] = json!(region.trim());
                                }
                            }
                        }
                    }
                }
            }
        }

        let result = json!({
            "count": profiles.len(),
            "profiles": profiles,
        });
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
                "size_human": format_size(obj.size().unwrap_or(0) as u64),
                "last_modified": obj.last_modified().map(|d| d.to_string()),
            }));
        }

        let result = json!({
            "profile": params.profile,
            "bucket": params.bucket,
            "prefix": prefix,
            "count": objects.len(),
            "objects": objects,
        });
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
