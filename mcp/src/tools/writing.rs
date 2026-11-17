//! Purpose: Creating, overwriting and appending to files, and creating
//! directories.

use super::{json_response, PathParam};
use crate::FfmServer;
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};
use serde::Deserialize;
use serde_json::json;
use std::fs::{self, OpenOptions};
use std::io::{self, Write as IoWrite};

#[cfg(test)]
mod tests;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateFileParams {
    /// Path for the new file
    pub path: String,
    /// File content (default: empty)
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WriteFileParams {
    /// Path to the file
    pub path: String,
    /// Content to write
    pub content: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AppendFileParams {
    /// Path to the file
    pub path: String,
    /// Content to append
    pub content: String,
}

#[tool_router(router = writing_tools, vis = "pub(super)")]
impl FfmServer {
    /// Create a new file with the given content. Fails if file already exists.
    #[tool(description = "Create a new file with the given content. Fails if file already exists.")]
    pub(crate) fn create_file(
        &self,
        Parameters(params): Parameters<CreateFileParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_readwrite()?;
        let path = self.safe_path_create(&params.path)?;
        let content = params.content.as_deref().unwrap_or("");

        create_parent(&path)?;

        // Atomic create — fails if file already exists (no TOCTOU race)
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| {
                if e.kind() == io::ErrorKind::AlreadyExists {
                    McpError::invalid_params(
                        format!("file already exists: {}", path.display()),
                        None,
                    )
                } else {
                    McpError::internal_error(e.to_string(), None)
                }
            })?;
        write_content(&mut file, content)?;

        let result = json!({ "created": path.display().to_string(), "size": content.len() });
        Ok(json_response(&result))
    }

    /// Write content to a file, creating or overwriting it.
    #[tool(description = "Write content to a file, creating or overwriting it.")]
    pub(crate) fn write_file(
        &self,
        Parameters(params): Parameters<WriteFileParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_readwrite()?;
        let path = self.safe_path_create(&params.path)?;

        if fs::symlink_metadata(&path).is_ok() {
            crate::fsops::write_replaced(&path, params.content.as_bytes())
                .map_err(crate::tools::io_error)?;
        } else {
            create_parent(&path)?;
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(crate::tools::io_error)?;
            write_content(&mut file, &params.content)?;
        }

        let result = json!({ "written": path.display().to_string(), "size": params.content.len() });
        Ok(json_response(&result))
    }

    /// Append content to an existing file.
    #[tool(
        description = "Append content to an existing file. Creates the file if it doesn't exist."
    )]
    pub(crate) fn append_file(
        &self,
        Parameters(params): Parameters<AppendFileParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_readwrite()?;
        let path = self.safe_path_create(&params.path)?;

        if let Ok(meta) = fs::symlink_metadata(&path) {
            if meta.file_type().is_symlink() || !meta.is_file() {
                return Err(McpError::invalid_params(
                    "target must be a regular file",
                    None,
                ));
            }
        }

        create_parent(&path)?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(crate::tools::io_error)?;

        write_content(&mut file, &params.content)?;

        let result =
            json!({ "appended": path.display().to_string(), "bytes_added": params.content.len() });
        Ok(json_response(&result))
    }

    /// Create a new directory (and parent directories if needed).
    #[tool(description = "Create a new directory (and parent directories if needed).")]
    pub(crate) fn create_directory(
        &self,
        Parameters(params): Parameters<PathParam>,
    ) -> Result<CallToolResult, McpError> {
        self.require_readwrite()?;
        let path = self.safe_path_create(&params.path)?;

        fs::create_dir_all(&path).map_err(crate::tools::io_error)?;

        let canonical = path.canonicalize().map_err(crate::tools::io_error)?;
        let result = json!({ "created": canonical.display().to_string() });
        Ok(json_response(&result))
    }
}

fn write_content(file: &mut fs::File, content: &str) -> Result<(), McpError> {
    file.write_all(content.as_bytes())
        .map_err(crate::tools::io_error)
}

fn create_parent(path: &std::path::Path) -> Result<(), McpError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(crate::tools::io_error)?;
    }
    Ok(())
}
