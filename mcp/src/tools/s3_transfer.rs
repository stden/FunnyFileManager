//! Purpose: Moving objects between S3 and the root: download, upload, delete
//! and copy.

use super::{json_response, s3::make_s3_client};
use crate::conversions::{format_size, saturating_u64};
use crate::FfmServer;
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};
use serde::Deserialize;
use serde_json::json;
use std::fs;

/// Maximum object size buffered into memory for a single `s3_download` call.
pub const MAX_S3_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;
const MAX_S3_OBJECT_BYTES: u64 = 64 * 1024 * 1024;

/// Returns `Err` when `content_length` exceeds [`MAX_S3_DOWNLOAD_BYTES`].
pub fn check_download_size(content_length: Option<i64>) -> Result<(), McpError> {
    if content_length.is_some_and(|len| len.unsigned_abs() > MAX_S3_DOWNLOAD_BYTES) {
        return Err(McpError::invalid_params("Object too large", None));
    }
    Ok(())
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct S3TransferParams {
    profile: String,
    bucket: String,
    key: String,
    local_path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct S3DeleteParams {
    profile: String,
    bucket: String,
    key: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct S3CopyParams {
    pub profile: String,
    pub source_bucket: String,
    pub source_key: String,
    pub dest_bucket: String,
    pub dest_key: String,
}

#[tool_router(router = s3_transfer_tools, vis = "pub(super)")]
impl FfmServer {
    /// Download a file from S3 to a local path.
    #[tool(description = "Download a file from S3 to a local path.")]
    pub(crate) async fn s3_download(
        &self,
        Parameters(params): Parameters<S3TransferParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_readwrite()?;
        let client = make_s3_client(&params.profile).await?;
        let _initial_path = self.safe_path_create(&params.local_path)?;

        let resp = client
            .get_object()
            .bucket(&params.bucket)
            .key(&params.key)
            .send()
            .await
            .map_err(internal_err)?;
        check_download_size(resp.content_length())?;

        let bytes = resp
            .body
            .collect()
            .await
            .map_err(internal_err)?
            .into_bytes();
        check_download_size(Some(i64::try_from(bytes.len()).unwrap_or(i64::MAX)))?;
        let local_path = self.safe_path_create(&params.local_path)?;
        write_downloaded(&local_path, &bytes).map_err(internal_err)?;
        Ok(transfer_result(&params, &local_path, bytes.len()))
    }

    /// Upload a local file to S3.
    #[tool(description = "Upload a local file to S3.")]
    pub(crate) async fn s3_upload(
        &self,
        Parameters(params): Parameters<S3TransferParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_readwrite()?;
        let client = make_s3_client(&params.profile).await?;
        let local_path = self.safe_file(&params.local_path)?;
        let meta = fs::metadata(&local_path).map_err(internal_err)?;
        if meta.len() > MAX_S3_OBJECT_BYTES {
            return Err(McpError::invalid_params("local file too large", None));
        }
        let bytes = fs::read(&local_path).map_err(internal_err)?;
        let size = bytes.len();
        client
            .put_object()
            .bucket(&params.bucket)
            .key(&params.key)
            .body(aws_sdk_s3::primitives::ByteStream::from(bytes))
            .send()
            .await
            .map_err(internal_err)?;
        Ok(transfer_result(&params, &local_path, size))
    }

    /// Delete an object from S3.
    #[tool(description = "Delete an object from S3.")]
    pub(crate) async fn s3_delete(
        &self,
        Parameters(params): Parameters<S3DeleteParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_readwrite()?;
        let client = make_s3_client(&params.profile).await?;
        client
            .delete_object()
            .bucket(&params.bucket)
            .key(&params.key)
            .send()
            .await
            .map_err(internal_err)?;
        Ok(json_response(
            &json!({ "bucket": params.bucket, "deleted": params.key }),
        ))
    }

    /// Copy an object within S3 (same or different bucket).
    #[tool(description = "Copy an S3 object within or between buckets.")]
    pub(crate) async fn s3_copy(
        &self,
        Parameters(params): Parameters<S3CopyParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_readwrite()?;
        if !valid_copy_source(&params.source_bucket, &params.source_key) {
            return Err(McpError::invalid_params("invalid source", None));
        }
        let client = make_s3_client(&params.profile).await?;
        let copy_source = format!("{}/{}", params.source_bucket, params.source_key);
        client
            .copy_object()
            .bucket(&params.dest_bucket)
            .key(&params.dest_key)
            .copy_source(&copy_source)
            .send()
            .await
            .map_err(internal_err)?;
        Ok(json_response(&json!({
            "source": format!("s3://{}/{}", params.source_bucket, params.source_key),
            "destination": format!("s3://{}/{}", params.dest_bucket, params.dest_key),
        })))
    }
}

fn internal_err(e: impl std::fmt::Display) -> McpError {
    McpError::internal_error(e.to_string(), None)
}

fn valid_copy_source(bucket: &str, key: &str) -> bool {
    !bucket.is_empty()
        && !key.is_empty()
        && !bucket
            .chars()
            .any(|c| c.is_control() || matches!(c, '?' | '#' | '/' | '%'))
        && !key
            .chars()
            .any(|c| c.is_control() || matches!(c, '?' | '#' | '%'))
}

fn transfer_result(
    params: &S3TransferParams,
    path: &std::path::Path,
    size: usize,
) -> CallToolResult {
    json_response(&json!({
        "bucket": params.bucket,
        "key": params.key,
        "local_path": path.display().to_string(),
        "size": size,
        "size_human": format_size(saturating_u64(size)),
    }))
}

fn write_downloaded(local_path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    if fs::symlink_metadata(local_path).is_ok() {
        return crate::fsops::write_replaced(local_path, bytes);
    }
    if let Some(parent) = local_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(local_path)?;
    std::io::Write::write_all(&mut file, bytes)
}
