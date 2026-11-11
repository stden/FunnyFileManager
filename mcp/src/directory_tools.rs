//! Directory ordering and available local or S3 drive reporting.

use super::{format_size, json_response, FfmServer, SortDirectoryParams};
use rmcp::{model::CallToolResult, ErrorData as McpError};
use serde_json::json;
use std::{fs, path::Path};

#[cfg(test)]
mod tests;

struct DirectoryItem {
    name: String,
    ext: String,
    is_dir: bool,
    size: u64,
}

#[derive(Clone, Copy)]
enum SortKey {
    Name,
    Extension,
    Size,
}

impl SortKey {
    fn parse(raw: &str) -> Self {
        match raw {
            "extension" | "ext" => Self::Extension,
            "size" => Self::Size,
            _ => Self::Name,
        }
    }
}

pub fn sort_directory(
    server: &FfmServer,
    params: &SortDirectoryParams,
) -> Result<CallToolResult, McpError> {
    let dir = server.safe_path(&params.path)?;

    if !dir.is_dir() {
        return Err(McpError::invalid_params(
            format!("not a directory: {}", dir.display()),
            None,
        ));
    }

    let mut items = directory_entries(&dir, |entry_path| {
        server
            .safe_path(&entry_path.to_string_lossy())
            .is_ok_and(|target| target.is_dir())
    })?;

    let sort_key = SortKey::parse(&params.sort);
    items.sort_by(|a, b| compare_entries(a, b, sort_key));
    let entries = into_directory_json(items);

    let result = json!({
        "path": dir.display().to_string(),
        "sort": params.sort,
        "count": entries.len(),
        "entries": entries,
    });
    Ok(json_response(&result))
}

fn into_directory_json(items: Vec<DirectoryItem>) -> Vec<serde_json::Value> {
    items
        .into_iter()
        .map(|item| {
            json!({
                "name": item.name,
                "type": if item.is_dir { "dir" } else { "file" },
                "size": item.size,
                "size_human": format_size(item.size),
                "extension": item.ext,
            })
        })
        .collect()
}

pub fn list_drives() -> CallToolResult {
    let mut drives = Vec::new();

    #[cfg(target_os = "windows")]
    for drive in cached_windows_drives(std::time::Duration::from_secs(5)) {
        drives.push(json!({ "type": "local", "path": drive }));
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

    drives.extend(
        super::s3_profiles::names()
            .into_iter()
            .map(|name| json!({ "type": "s3", "profile": name })),
    );

    json_response(&json!({ "drives": drives }))
}

#[cfg(target_os = "windows")]
struct CachedDrives {
    sampled_at: std::time::Instant,
    drives: Vec<String>,
}

#[cfg(target_os = "windows")]
type DriveCache = std::sync::Mutex<Option<CachedDrives>>;

#[cfg(target_os = "windows")]
fn cached_windows_drives(ttl: std::time::Duration) -> Vec<String> {
    use std::sync::{OnceLock, PoisonError};
    use std::time::Instant;

    static CACHE: OnceLock<DriveCache> = OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(None));
    if let Some(cached) = cache
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .filter(|cached| cached.sampled_at.elapsed() < ttl)
    {
        return cached.drives.clone();
    }
    let discovered = probe_windows_drives();
    *cache.lock().unwrap_or_else(PoisonError::into_inner) = Some(CachedDrives {
        sampled_at: Instant::now(),
        drives: discovered.clone(),
    });
    discovered
}

#[cfg(target_os = "windows")]
fn probe_windows_drives() -> Vec<String> {
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(26);
        for letter in 'A'..='Z' {
            handles.push(scope.spawn(move || {
                let drive = format!("{letter}:\\");
                Path::new(&drive).exists().then_some(drive)
            }));
        }
        handles
            .into_iter()
            .filter_map(|handle| handle.join().ok().flatten())
            .collect()
    })
}

fn compare_entries(a: &DirectoryItem, b: &DirectoryItem, sort_key: SortKey) -> std::cmp::Ordering {
    let directories = b.is_dir.cmp(&a.is_dir);
    if directories != std::cmp::Ordering::Equal {
        return directories;
    }
    match sort_key {
        SortKey::Extension => a.ext.cmp(&b.ext).then_with(|| a.name.cmp(&b.name)),
        SortKey::Size => b.size.cmp(&a.size),
        SortKey::Name => a.name.cmp(&b.name),
    }
}

fn directory_entries(
    path: &Path,
    is_confined_dir: impl Fn(&Path) -> bool,
) -> Result<Vec<DirectoryItem>, McpError> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(path).map_err(crate::tools::io_error)? {
        let entry = entry.map_err(crate::tools::io_error)?;
        let meta = entry.metadata().map_err(crate::tools::io_error)?;
        let is_dir =
            meta.is_dir() || (meta.file_type().is_symlink() && is_confined_dir(&entry.path()));
        let name = entry.file_name().to_string_lossy().into_owned();
        let ext = Path::new(&name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_owned();
        entries.push(DirectoryItem {
            name,
            ext,
            is_dir,
            size: meta.len(),
        });
    }

    Ok(entries)
}
