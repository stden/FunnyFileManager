//! Directory ordering and available local or S3 drive reporting.

use super::{format_size, json_response, FfmServer, SortDirectoryParams};
use rmcp::{model::CallToolResult, ErrorData as McpError};
use serde_json::json;
use std::{fs, path::Path};

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

    let mut entries = directory_entries(&path)?;

    entries.sort_by(|a, b| compare_entries(a, b, &params.sort));

    let result = json!({
        "path": path.display().to_string(),
        "sort": params.sort,
        "count": entries.len(),
        "entries": entries,
    });
    Ok(json_response(&result))
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

fn compare_entries(a: &serde_json::Value, b: &serde_json::Value, sort: &str) -> std::cmp::Ordering {
    let directories = (b["type"] == "dir").cmp(&(a["type"] == "dir"));
    if directories != std::cmp::Ordering::Equal {
        return directories;
    }
    match sort {
        "extension" | "ext" => a["extension"]
            .as_str()
            .unwrap_or("")
            .cmp(b["extension"].as_str().unwrap_or(""))
            .then_with(|| compare_names(a, b)),
        "size" => b["size"]
            .as_u64()
            .unwrap_or(0)
            .cmp(&a["size"].as_u64().unwrap_or(0)),
        _ => compare_names(a, b),
    }
}

fn compare_names(a: &serde_json::Value, b: &serde_json::Value) -> std::cmp::Ordering {
    a["name"]
        .as_str()
        .unwrap_or("")
        .cmp(b["name"].as_str().unwrap_or(""))
}

fn directory_entries(path: &Path) -> Result<Vec<serde_json::Value>, McpError> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(path).map_err(|e| McpError::internal_error(e.to_string(), None))? {
        let entry = entry.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let meta = entry
            .metadata()
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let is_dir = meta.is_dir() || (meta.file_type().is_symlink() && entry.path().is_dir());
        let name = entry.file_name().to_string_lossy().to_string();
        let ext = Path::new(&name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_owned();
        entries.push(json!({
            "name": name,
            "type": if is_dir { "dir" } else { "file" },
            "size": meta.len(),
            "size_human": format_size(meta.len()),
            "extension": ext,
        }));
    }

    Ok(entries)
}
