//! Purpose: Copying, moving and deleting one entry, reproducing a link as a
//! link rather than following it, and never replacing what is already at
//! the destination.

use super::{default_true, json_response};
use crate::fsops::{self, Moved, Purpose};
use crate::FfmServer;
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CopyMoveParams {
    /// Source path
    pub source: String,
    /// Destination path
    pub destination: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteParams {
    /// Path to delete
    path: String,
    /// Allow recursive directory deletion (default: true)
    #[serde(default = "default_true")]
    recursive: bool,
}

#[tool_router(router = transfer_tools, vis = "pub(super)")]
impl FfmServer {
    /// Copy a file or directory to a destination that does not exist yet.
    #[tool(
        description = "Copy a file or directory to a destination that does not exist yet. Directories are copied recursively; nothing is overwritten or merged."
    )]
    pub(crate) fn copy(
        &self,
        Parameters(params): Parameters<CopyMoveParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_readwrite()?;
        // Resolve the link itself, never its target: canonicalizing would hand
        // back what the link points at, so copying a link would copy that file's
        // contents instead of reproducing the link.
        let source = self.safe_path_nofollow(&params.source)?;
        let dest = self.safe_path_create(&params.destination)?;
        require_source(&source)?;
        make_parent(&dest)?;
        fsops::copy_entry(&source, &dest, Purpose::Copy).map_err(|e| transfer_error(&e))?;

        let result =
            json!({ "copied": source.display().to_string(), "to": dest.display().to_string() });
        Ok(json_response(&result))
    }

    /// Move or rename a file or directory to a destination that does not
    /// exist yet.
    #[tool(
        description = "Move or rename a file or directory to a destination that does not exist yet. Across filesystems it copies, then removes only what the copy holds and reports what stayed."
    )]
    pub(crate) fn move_rename(
        &self,
        Parameters(params): Parameters<CopyMoveParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_readwrite()?;
        // Resolve the link itself: `rename` moves a link without following it,
        // so canonicalizing here would rename the *target* out from under the
        // link — and a copy fallback would delete that target instead.
        let source = self.safe_path_nofollow(&params.source)?;
        if source == self.root {
            return Err(McpError::invalid_params(
                "cannot delete or move root directory",
                None,
            ));
        }
        let dest = self.safe_path_create(&params.destination)?;
        require_source(&source)?;
        make_parent(&dest)?;

        let (moved, to) = (source.display().to_string(), dest.display().to_string());
        match fsops::move_entry(&source, &dest).map_err(|e| transfer_error(&e))? {
            Moved::Whole => Ok(json_response(&json!({ "moved": moved, "to": to }))),
            // Both are true, and the caller acts on each: the whole copy is
            // at `to`, and what stayed is named — never "the move failed",
            // which a caller once answered by deleting the "extra" copy.
            Moved::Leaving(left) => Err(McpError::internal_error(
                format!("moved to {to}: {left}"),
                Some(json!({
                    "moved": moved,
                    "to": to,
                    "left_behind": {
                        "path": left.path.display().to_string(),
                        "reason": left.reason.to_string(),
                    },
                })),
            )),
        }
    }

    /// Delete a file or directory (recursively for directories).
    #[tool(description = "Delete a file or directory (recursively for directories).")]
    pub(crate) fn delete(
        &self,
        Parameters(params): Parameters<DeleteParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_readwrite()?;
        // Resolve the link itself, never its target: deleting a link must not
        // destroy what it points at, and a dangling link stays deletable.
        let path = self.safe_path_nofollow(&params.path)?;
        if path == self.root {
            return Err(McpError::invalid_params(
                "cannot delete or move root directory",
                None,
            ));
        }

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
}

/// `symlink_metadata`, not `exists()`: `exists()` follows the link, so a
/// dangling one would be reported as missing although it is right there —
/// and it is copyable and movable as a link.
fn require_source(source: &Path) -> Result<(), McpError> {
    if fs::symlink_metadata(source).is_err() {
        return Err(McpError::invalid_params(
            format!("source not found: {}", source.display()),
            None,
        ));
    }
    Ok(())
}

fn make_parent(dest: &Path) -> Result<(), McpError> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| McpError::internal_error(e.to_string(), None))?;
    }
    Ok(())
}

/// A destination that is taken, or a source sent into itself, is the
/// caller's to change; anything else is the filesystem's answer.
fn transfer_error(error: &io::Error) -> McpError {
    match error.kind() {
        io::ErrorKind::AlreadyExists | io::ErrorKind::InvalidInput => {
            McpError::invalid_params(error.to_string(), None)
        }
        _ => McpError::internal_error(error.to_string(), None),
    }
}
