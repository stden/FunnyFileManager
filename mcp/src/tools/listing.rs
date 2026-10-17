//! Purpose: Directory listings and the metadata of one entry.

use super::{json_response, PathParam};
use crate::conversions::format_size;
use crate::{directory_tools, FfmServer};
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SortDirectoryParams {
    /// Directory path to list
    pub path: String,
    /// Sort mode: "name" (default), "extension", or "size"
    #[serde(default = "default_sort_mode")]
    pub sort: String,
}

fn default_sort_mode() -> String {
    "name".to_owned()
}

#[tool_router(router = listing_tools, vis = "pub(super)")]
impl FfmServer {
    /// List files and directories at the given path.
    /// Returns name, type (file/dir), and size for each entry.
    #[tool(
        description = "List files and directories at the given path. Returns name, type, and size."
    )]
    pub(crate) fn list_directory(
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
            let is_dir = meta.is_dir() || (meta.file_type().is_symlink() && entry.path().is_dir());
            let name = entry.file_name().to_string_lossy().to_string();
            entries.push(json!({
                "name": name,
                "type": if is_dir { "dir" } else { "file" },
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
    pub(crate) fn sort_directory(
        &self,
        Parameters(params): Parameters<SortDirectoryParams>,
    ) -> Result<CallToolResult, McpError> {
        directory_tools::sort_directory(self, &params)
    }

    /// Get detailed info about a file or directory.
    #[tool(
        description = "Get detailed info about a file or directory: size, type, permissions, modified time."
    )]
    pub(crate) fn get_file_info(
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
