//! Directory ordering and available local or S3 drive reporting.

use super::{format_size, json_response, FfmServer, SortDirectoryParams};
use rmcp::{model::CallToolResult, ErrorData as McpError};
use serde_json::json;
use std::{fs, path::Path};

struct DirectoryItem {
    name: String,
    ext: String,
    is_dir: bool,
    size: u64,
}

pub fn sort_directory(
    server: &FfmServer,
    params: &SortDirectoryParams,
) -> Result<CallToolResult, McpError> {
    let path = server.safe_path(&params.path)?;

    if !path.is_dir() {
        return Err(McpError::invalid_params(
            format!("not a directory: {}", path.display()),
            None,
        ));
    }

    let mut items = directory_entries(&path, |entry_path| {
        server
            .safe_path(&entry_path.to_string_lossy())
            .is_ok_and(|target| target.is_dir())
    })?;

    items.sort_by(|a, b| compare_entries(a, b, &params.sort));
    let entries = into_directory_json(items);

    let result = json!({
        "path": path.display().to_string(),
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
    {
        for letter in 'A'..='Z' {
            let drive = format!("{letter}:\\");
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

    drives.extend(
        super::s3_profiles::names()
            .into_iter()
            .map(|name| json!({ "type": "s3", "profile": name })),
    );

    json_response(&json!({ "drives": drives }))
}

fn compare_entries(a: &DirectoryItem, b: &DirectoryItem, sort: &str) -> std::cmp::Ordering {
    let directories = b.is_dir.cmp(&a.is_dir);
    if directories != std::cmp::Ordering::Equal {
        return directories;
    }
    match sort {
        "extension" | "ext" => a.ext.cmp(&b.ext).then_with(|| a.name.cmp(&b.name)),
        "size" => b.size.cmp(&a.size),
        _ => a.name.cmp(&b.name),
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
