//! Purpose: AWS S3 browsing — profiles, buckets and one level of objects —
//! and the client every S3 tool connects with.

use super::json_response;
use crate::conversions::{format_size, object_size};
use crate::{s3_listing, s3_profiles, FfmServer};
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct S3ProfileParam {
    /// AWS profile name from ~/.aws/credentials
    profile: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct S3ListObjectsParams {
    /// AWS profile name
    profile: String,
    /// S3 bucket name
    bucket: String,
    /// Prefix/folder to list (default: root)
    #[serde(default)]
    prefix: Option<String>,
}

pub async fn make_s3_client(profile: &str) -> Result<aws_sdk_s3::Client, McpError> {
    use aws_config::BehaviorVersion;

    let config = aws_config::defaults(BehaviorVersion::latest())
        .credentials_provider(
            aws_config::profile::ProfileFileCredentialsProvider::builder()
                .profile_name(profile)
                .build(),
        )
        .region(
            aws_config::meta::region::RegionProviderChain::default_provider().or_else("us-east-1"),
        )
        .load()
        .await;

    Ok(aws_sdk_s3::Client::new(&config))
}

#[tool_router(router = s3_tools, vis = "pub(super)")]
impl FfmServer {
    /// List AWS profiles from ~/.aws/credentials.
    #[tool(description = "List AWS profiles from ~/.aws/credentials file.")]
    pub(crate) fn s3_list_profiles() -> CallToolResult {
        s3_profiles::list()
    }

    /// List S3 buckets for a given AWS profile.
    #[tool(description = "List S3 buckets for a given AWS profile.")]
    pub(crate) async fn s3_list_buckets(
        &self,
        Parameters(params): Parameters<S3ProfileParam>,
    ) -> Result<CallToolResult, McpError> {
        let client = make_s3_client(&params.profile).await?;

        let resp = client
            .list_buckets()
            .send()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let buckets: Vec<serde_json::Value> = resp
            .buckets()
            .iter()
            .map(|b| {
                json!({
                    "name": b.name().unwrap_or(""),
                    "creation_date": b.creation_date().map(ToString::to_string),
                })
            })
            .collect();

        let result = json!({
            "profile": params.profile,
            "count": buckets.len(),
            "buckets": buckets,
        });
        Ok(json_response(&result))
    }

    /// List objects in an S3 bucket.
    #[tool(description = "List objects in an S3 bucket with optional prefix filter.")]
    pub(crate) async fn s3_list_objects(
        &self,
        Parameters(params): Parameters<S3ListObjectsParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = make_s3_client(&params.profile).await?;
        let prefix = params.prefix.as_deref().unwrap_or("");
        let listed = s3_listing::list_level(&client, &params.bucket, prefix)
            .await
            .map_err(|error| McpError::internal_error(error, None))?;
        let objects: Vec<_> = listed.iter().map(listed_object).collect();
        let result = json!({
            "profile": params.profile,
            "bucket": params.bucket,
            "prefix": prefix,
            "count": objects.len(),
            "objects": objects,
        });
        Ok(json_response(&result))
    }
}

fn listed_object(object: &s3_listing::ListedObject) -> serde_json::Value {
    json!({
        "key": object.key,
        "type": if object.folder { "folder" } else { "file" },
        "size": object.size,
        "size_human": format_size(object_size(Some(object.size))),
        "last_modified": object.modified,
    })
}
