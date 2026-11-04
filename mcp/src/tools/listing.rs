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

#[cfg(test)]
mod tests;

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

struct ListedItem {
    name: String,
    is_dir: bool,
    size: u64,
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
        let mut items = self.collect_listing(&path)?;
        sort_listing(&mut items);
        let entries: Vec<serde_json::Value> = items
            .into_iter()
            .map(|item| {
                json!({
                    "name": item.name,
                    "type": if item.is_dir { "dir" } else { "file" },
                    "size": item.size,
                    "size_human": format_size(item.size),
                })
            })
            .collect();

        let result = json!({
            "path": path.display().to_string(),
            "count": entries.len(),
            "entries": entries,
        });
        Ok(json_response(&result))
    }

    fn collect_listing(&self, path: &Path) -> Result<Vec<ListedItem>, McpError> {
        let mut items = Vec::new();
        for entry in fs::read_dir(path).map_err(crate::tools::io_error)? {
            let entry = entry.map_err(crate::tools::io_error)?;
            let meta = entry.metadata().map_err(crate::tools::io_error)?;
            let is_dir = meta.is_dir()
                || (meta.file_type().is_symlink()
                    && self
                        .safe_path(&entry.path().to_string_lossy())
                        .is_ok_and(|target| target.is_dir()));
            items.push(ListedItem {
                name: entry.file_name().to_string_lossy().into_owned(),
                is_dir,
                size: meta.len(),
            });
        }
        Ok(items)
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
        let path = self.safe_path_nofollow(&params.path)?;
        let meta = fs::symlink_metadata(&path).map_err(crate::tools::io_error)?;

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

fn sort_listing(entries: &mut [ListedItem]) {
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
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

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        info["mode"] = json!(format!("{:o}", meta.permissions().mode()));
    }

    if meta.is_symlink() {
        if let Ok(target) = fs::read_link(path) {
            info["symlink_target"] = json!(target.display().to_string());
        }
    }
}
