//! AWS S3 integration module for `FunnyFileManager`
//!
//! Provides S3 bucket browsing as virtual drives.

use aws_config::meta::region::RegionProviderChain;
use aws_config::profile::{ProfileFileCredentialsProvider, ProfileFileRegionProvider};
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;
use std::io;
use std::panic::{catch_unwind, AssertUnwindSafe};

mod listing;
mod pages;
mod profiles;
mod transfer;
mod virtual_path;

pub use listing::{S3Bucket, S3Object};
use pages::{collect_pages, folder_prefix};
pub use profiles::{list_aws_profiles, AwsProfile};
pub use virtual_path::VirtualPath;

/// S3 client wrapper with blocking operations for TUI
pub struct S3Client {
    client: Client,
    runtime: tokio::runtime::Runtime,
    pub profile: String,
    pub current_bucket: Option<String>,
    pub current_prefix: String,
}

impl S3Client {
    /// Create a new S3 client for the given profile
    pub fn new(profile_name: &str) -> io::Result<Self> {
        let runtime = tokio::runtime::Runtime::new().map_err(io::Error::other)?;

        let client = catch_unwind(AssertUnwindSafe(|| {
            runtime.block_on(async { configured_client(profile_name).await })
        }))
        .map_err(|panic_payload| {
            let message = if let Some(message) = panic_payload.downcast_ref::<&str>() {
                (*message).to_owned()
            } else if let Some(message) = panic_payload.downcast_ref::<String>() {
                message.clone()
            } else {
                "unknown panic during S3 client initialization".to_owned()
            };
            io::Error::other(format!("failed to initialize S3 client: {message}"))
        })?;

        Ok(Self {
            client,
            runtime,
            profile: profile_name.to_owned(),
            current_bucket: None,
            current_prefix: String::new(),
        })
    }

    /// List all S3 buckets for this profile
    pub fn list_buckets(&self) -> io::Result<Vec<S3Bucket>> {
        self.runtime.block_on(async {
            let resp = self
                .client
                .list_buckets()
                .send()
                .await
                .map_err(|e| io::Error::other(e.to_string()))?;

            let buckets = resp
                .buckets()
                .iter()
                .map(|b| S3Bucket {
                    name: b.name().unwrap_or_default().to_owned(),
                    creation_date: b.creation_date().map(ToString::to_string),
                })
                .collect();

            Ok(buckets)
        })
    }

    /// List one folder level. A prefix without a trailing slash is a folder
    /// name (`data` lists `data/`). Every truncated page is followed.
    pub fn list_objects(&self, bucket: &str, prefix: &str) -> io::Result<Vec<S3Object>> {
        let prefix = folder_prefix(prefix);
        self.runtime
            .block_on(collect_pages(&self.client, bucket, &prefix))
    }
}

#[cfg(test)]
mod tests;

async fn configured_client(profile_name: &str) -> Client {
    let region_provider = RegionProviderChain::first_try(
        ProfileFileRegionProvider::builder()
            .profile_name(profile_name)
            .build(),
    )
    .or_default_provider()
    .or_else("us-east-1");

    let credentials_provider = ProfileFileCredentialsProvider::builder()
        .profile_name(profile_name)
        .build();

    let config = aws_config::defaults(BehaviorVersion::latest())
        .profile_name(profile_name)
        .credentials_provider(credentials_provider)
        .region(region_provider)
        .load()
        .await;

    Client::new(&config)
}
