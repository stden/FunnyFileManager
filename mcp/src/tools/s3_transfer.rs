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

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct S3TransferParams {
    /// AWS profile name
    profile: String,
    /// S3 bucket name
    bucket: String,
    /// S3 object key
    key: String,
    /// Local file path
    local_path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct S3DeleteParams {
    /// AWS profile name
    profile: String,
    /// S3 bucket name
    bucket: String,
    /// S3 object key to delete
    key: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct S3CopyParams {
    /// AWS profile name
    profile: String,
    /// Source bucket
    source_bucket: String,
    /// Source object key
    source_key: String,
    /// Destination bucket
    dest_bucket: String,
    /// Destination object key
    dest_key: String,
}

#[tool_router(router = s3_transfer_tools, vis = "pub(super)")]
impl FfmServer {
    /// Download a file from S3 to a local path.
    #[tool(description = "Download a file from S3 to a local path.")]
    pub(crate) async fn s3_download(
        &self,
        Parameters(params): Parameters<S3TransferParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = make_s3_client(&params.profile).await?;
        let local_path = self.safe_path_create(&params.local_path)?;

        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        let resp = client
            .get_object()
            .bucket(&params.bucket)
            .key(&params.key)
            .send()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let bytes = resp
            .body
            .collect()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .into_bytes();

        fs::write(&local_path, &bytes)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({
            "bucket": params.bucket,
            "key": params.key,
            "local_path": local_path.display().to_string(),
            "size": bytes.len(),
            "size_human": format_size(saturating_u64(bytes.len())),
        });
        Ok(json_response(&result))
    }

    /// Upload a local file to S3.
    #[tool(description = "Upload a local file to S3.")]
    pub(crate) async fn s3_upload(
        &self,
        Parameters(params): Parameters<S3TransferParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = make_s3_client(&params.profile).await?;
        let local_path = self.safe_path(&params.local_path)?;

        let bytes =
            fs::read(&local_path).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let body = aws_sdk_s3::primitives::ByteStream::from(bytes.clone());

        client
            .put_object()
            .bucket(&params.bucket)
            .key(&params.key)
            .body(body)
            .send()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({
            "bucket": params.bucket,
            "key": params.key,
            "local_path": local_path.display().to_string(),
            "size": bytes.len(),
            "size_human": format_size(saturating_u64(bytes.len())),
        });
        Ok(json_response(&result))
    }

    /// Delete an object from S3.
    #[tool(description = "Delete an object from S3.")]
    pub(crate) async fn s3_delete(
        &self,
        Parameters(params): Parameters<S3DeleteParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = make_s3_client(&params.profile).await?;

        client
            .delete_object()
            .bucket(&params.bucket)
            .key(&params.key)
            .send()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({
            "bucket": params.bucket,
            "deleted": params.key,
        });
        Ok(json_response(&result))
    }

    /// Copy an object within S3 (same or different bucket).
    #[tool(description = "Copy an S3 object within or between buckets.")]
    pub(crate) async fn s3_copy(
        &self,
        Parameters(params): Parameters<S3CopyParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = make_s3_client(&params.profile).await?;

        let copy_source = format!("{}/{}", params.source_bucket, params.source_key);
        client
            .copy_object()
            .bucket(&params.dest_bucket)
            .key(&params.dest_key)
            .copy_source(&copy_source)
            .send()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = json!({
            "source": format!("s3://{}/{}", params.source_bucket, params.source_key),
            "destination": format!("s3://{}/{}", params.dest_bucket, params.dest_key),
        });
        Ok(json_response(&result))
    }
}
