//! Purpose: Copying, moving and deleting one entry, reproducing a link as a
//! link rather than following it.

use super::{default_true, json_response};
use crate::{fsops, FfmServer};
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
    pub path: String,
    /// Allow recursive directory deletion (default: true)
    #[serde(default = "default_true")]
    pub recursive: bool,
}

#[tool_router(router = transfer_tools, vis = "pub(super)")]
impl FfmServer {
    /// Copy a file or directory to a destination. Directories are copied recursively.
    #[tool(
        description = "Copy a file or directory to a destination. Directories are copied recursively."
    )]
    pub(crate) fn copy(
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
        Ok(json_response(&result))
    }

    /// Move or rename a file or directory.
    #[tool(description = "Move or rename a file or directory.")]
    pub(crate) fn move_rename(
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
        Ok(json_response(&result))
    }

    /// Delete a file or directory (recursively for directories).
    #[tool(description = "Delete a file or directory (recursively for directories).")]
    pub(crate) fn delete(
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
        Ok(json_response(&result))
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
pub fn copy_entry(src: &Path, dst: &Path) -> io::Result<()> {
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
