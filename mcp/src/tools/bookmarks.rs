//! Purpose: Named bookmarks for directories, kept in `~/.ffm/bookmarks.json`.

use super::{json_response, system::dirs_home};
use crate::FfmServer;
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BookmarkParams {
    /// Bookmark name/key
    name: String,
    /// Path to bookmark (required for `set_bookmark`)
    #[serde(default)]
    path: Option<String>,
}

#[tool_router(router = bookmark_tools, vis = "pub(super)")]
impl FfmServer {
    /// Save a directory path as a named bookmark.
    #[tool(description = "Save a directory path as a named bookmark for quick navigation.")]
    pub(crate) fn set_bookmark(
        &self,
        Parameters(params): Parameters<BookmarkParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_readwrite()?;
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
    pub(crate) fn get_bookmark(
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
    pub(crate) fn list_bookmarks() -> CallToolResult {
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
    pub(crate) fn delete_bookmark(
        &self,
        Parameters(params): Parameters<BookmarkParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_readwrite()?;
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
