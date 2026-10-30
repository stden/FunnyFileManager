//! Purpose: Copying, moving and deleting many entries in one call, each
//! failure reported beside its entry.

use super::{default_true, json_response};
use crate::fsops::{self, copy_entry, Moved, Purpose};
use crate::naming::{entry_name, join_child};
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
pub struct BatchParams {
    /// List of source paths
    pub paths: Vec<String>,
    /// Destination directory (for copy/move)
    #[serde(default)]
    pub destination: Option<String>,
    /// Allow recursive directory operations (default: true)
    #[serde(default = "default_true")]
    pub recursive: bool,
}

#[tool_router(router = batch_tools, vis = "pub(super)")]
impl FfmServer {
    /// Copy multiple files/directories into a destination directory.
    #[tool(
        description = "Copy multiple files/directories into a destination directory. A name already taken there is reported, never overwritten or merged."
    )]
    pub(crate) fn batch_copy(
        &self,
        Parameters(params): Parameters<BatchParams>,
    ) -> Result<CallToolResult, McpError> {
        let dest = prepare_batch_dest(self, &params)?;

        let mut copied = Vec::new();
        let mut errors = Vec::new();

        // Exactly the pair `copy` uses: resolve without following the final
        // link, then let `copy_entry` reproduce whatever the source really is.
        // `sanitize_path` would canonicalize the link away and copy in the
        // contents of the file it points at.
        let copy_one = |src_path: &str| -> Result<String, String> {
            let source = self.safe_path_nofollow_entry(src_path)?;
            let name = entry_name(&source);
            let target = join_child(&dest, &name)?;
            ensure_not_self_copy(&source, &target)?;
            fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
            copy_entry(&source, &target, Purpose::Copy).map_err(|e| e.to_string())?;
            Ok(name)
        };

        for src_path in &params.paths {
            match copy_one(src_path) {
                Ok(name) => copied.push(name),
                Err(error) => errors.push(json!({"path": src_path, "error": error})),
            }
        }
        if params.paths.is_empty() {
            fs::create_dir_all(&dest).map_err(|e| McpError::internal_error(e.to_string(), None))?;
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

    /// Move multiple files/directories into a destination directory.
    #[tool(
        description = "Move multiple files/directories into a destination directory. A name already taken there is reported, never replaced; across filesystems each entry is copied, then only what the copy holds is removed and what stayed is reported beside it."
    )]
    pub(crate) fn batch_move(
        &self,
        Parameters(params): Parameters<BatchParams>,
    ) -> Result<CallToolResult, McpError> {
        let dest = prepare_batch_dest(self, &params)?;

        let mut moved = Vec::new();
        let mut errors = Vec::new();

        for src_path in &params.paths {
            match move_batch_entry(self, &dest, src_path) {
                Ok((name, Moved::Whole)) => moved.push(name),
                Ok((name, Moved::Leaving(left))) => {
                    moved.push(name);
                    errors.push(json!({"path": src_path, "error": left.to_string()}));
                }
                Err(error) => errors.push(json!({"path": src_path, "error": error})),
            }
        }
        if params.paths.is_empty() {
            fs::create_dir_all(&dest).map_err(|e| McpError::internal_error(e.to_string(), None))?;
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
    pub(crate) fn batch_delete(
        &self,
        Parameters(params): Parameters<BatchParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_readwrite()?;
        let mut deleted = Vec::new();
        let mut errors = Vec::new();

        let delete_one = |src_path: &str| -> Result<String, String> {
            let path = self.safe_path_nofollow_entry(src_path)?;
            if path == self.root {
                return Err("cannot delete or move root directory".to_owned());
            }
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
        Ok(json_response(&result))
    }
}

/// Validate write access and resolve the required `destination` field through
/// the server root.
fn prepare_batch_dest(server: &FfmServer, params: &BatchParams) -> Result<PathBuf, McpError> {
    let dest = server.safe_path_create(
        params
            .destination
            .as_deref()
            .ok_or_else(|| McpError::invalid_params("destination is required", None))?,
    )?;
    server.require_readwrite()?;
    Ok(dest)
}

/// One entry of a batch move: the name it lands under, and what the move
/// achieved.
fn move_batch_entry(
    server: &FfmServer,
    dest: &Path,
    src_path: &str,
) -> Result<(String, Moved), String> {
    let source = server.safe_path_nofollow_entry(src_path)?;
    if source == server.root {
        return Err("cannot delete or move root directory".to_owned());
    }
    let name = entry_name(&source);
    let target = join_child(dest, &name)?;
    ensure_not_self_copy(&source, &target)?;
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let moved = fsops::move_entry(&source, &target).map_err(|e| e.to_string())?;
    Ok((name, moved))
}

fn ensure_not_self_copy(source: &Path, target: &Path) -> Result<(), String> {
    if target.starts_with(source) && fs::symlink_metadata(source).is_ok_and(|m| m.is_dir()) {
        return Err(format!(
            "cannot copy a directory into itself: {}",
            source.display()
        ));
    }
    Ok(())
}
