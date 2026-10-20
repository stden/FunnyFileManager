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
use std::path::PathBuf;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BookmarkParams {
    /// Bookmark name/key
    pub name: String,
    /// Path to bookmark (required for `set_bookmark`)
    #[serde(default)]
    pub path: Option<String>,
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
        if params.name.is_empty()
            || params.name.len() > 256
            || params.name.chars().any(char::is_control)
        {
            return Err(McpError::invalid_params("invalid bookmark name", None));
        }

        let path_str = params
            .path
            .as_deref()
            .ok_or_else(|| McpError::invalid_params("path is required for set_bookmark", None))?;
        let path = self.safe_dir(path_str)?;

        let mut bookmarks = load_bookmarks();
        if !bookmarks.contains_key(&params.name) && bookmarks.len() >= 1000 {
            return Err(McpError::invalid_params(
                "maximum of 1000 bookmarks reached",
                None,
            ));
        }
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
        &self,
        Parameters(params): Parameters<BookmarkParams>,
    ) -> Result<CallToolResult, McpError> {
        let bookmarks = load_bookmarks();
        match bookmarks.get(&params.name) {
            Some(path) => {
                let path_str = path.as_str().unwrap_or("");
                let resolved = self.safe_path_create(path_str)?;
                let result = json!({
                    "bookmark": params.name,
                    "path": path,
                    "exists": resolved.exists(),
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
    pub(crate) fn list_bookmarks(&self) -> CallToolResult {
        let bookmarks = load_bookmarks();
        let entries = self.visible_bookmarks(&bookmarks);

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
        self.remove_confined_bookmark(&mut bookmarks, &params.name)?;
        save_bookmarks(&bookmarks).map_err(|e| McpError::internal_error(e, None))?;

        let result = json!({ "deleted_bookmark": params.name });
        Ok(json_response(&result))
    }

    pub(crate) fn visible_bookmarks(
        &self,
        bookmarks: &serde_json::Map<String, serde_json::Value>,
    ) -> Vec<serde_json::Value> {
        bookmarks
            .iter()
            .filter_map(|(name, path)| {
                let path_str = path.as_str()?;
                let resolved = self.safe_path_create(path_str).ok()?;
                Some(json!({
                    "name": name,
                    "path": path,
                    "exists": resolved.exists(),
                }))
            })
            .collect()
    }

    pub(crate) fn remove_confined_bookmark(
        &self,
        bookmarks: &mut serde_json::Map<String, serde_json::Value>,
        name: &str,
    ) -> Result<(), McpError> {
        let Some(stored) = bookmarks.get(name) else {
            return Err(McpError::invalid_params(
                format!("bookmark not found: {name}"),
                None,
            ));
        };
        let path_str = stored.as_str().unwrap_or("");
        self.safe_path_create(path_str)?;
        bookmarks.remove(name);
        Ok(())
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
    if fs::symlink_metadata(&path).is_ok() {
        crate::fsops::write_replaced(&path, json.as_bytes()).map_err(|e| e.to_string())
    } else {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        std::io::Write::write_all(&mut file, json.as_bytes()).map_err(|e| e.to_string())
    }
}
