//! AWS S3 integration module for `FunnyFileManager`
//!
//! Provides S3 bucket browsing as virtual drives.

use aws_config::meta::region::RegionProviderChain;
use aws_config::profile::ProfileFileCredentialsProvider;
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;
use std::io;
use std::io::Write as _;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

mod listing;
mod profiles;
mod virtual_path;

use listing::{file_from_object, folder_from_prefix};
pub use listing::{S3Bucket, S3Object};
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
                (*message).to_string()
            } else if let Some(message) = panic_payload.downcast_ref::<String>() {
                message.clone()
            } else {
                "unknown panic during S3 client initialization".to_string()
            };
            io::Error::other(format!("failed to initialize S3 client: {message}"))
        })?;

        Ok(Self {
            client,
            runtime,
            profile: profile_name.to_string(),
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
                    name: b.name().unwrap_or_default().to_string(),
                    creation_date: b.creation_date().map(std::string::ToString::to_string),
                })
                .collect();

            Ok(buckets)
        })
    }

    /// List objects in a bucket with optional prefix (folder path)
    pub fn list_objects(&self, bucket: &str, prefix: &str) -> io::Result<Vec<S3Object>> {
        self.runtime.block_on(async {
            let mut request = self.client.list_objects_v2().bucket(bucket).delimiter("/");

            if !prefix.is_empty() {
                request = request.prefix(prefix);
            }

            let resp = request
                .send()
                .await
                .map_err(|e| io::Error::other(e.to_string()))?;

            let folders = resp
                .common_prefixes()
                .iter()
                .filter_map(|entry| folder_from_prefix(entry.prefix()?, prefix));
            let files = resp
                .contents()
                .iter()
                .filter_map(|object| file_from_object(object, prefix));
            Ok(folders.chain(files).collect())
        })
    }

    /// Download an object to a local path
    pub fn download_object(&self, bucket: &str, key: &str, local_path: &Path) -> io::Result<()> {
        self.runtime.block_on(async {
            let resp = self
                .client
                .get_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| io::Error::other(e.to_string()))?;

            let data = resp
                .body
                .collect()
                .await
                .map_err(|e| io::Error::other(e.to_string()))?;

            let bytes = data.into_bytes();
            // Stage in a *unique* temp file in the destination's own directory,
            // so the final rename stays atomic. `with_extension("part")` used to
            // replace the extension instead of appending: downloading
            // `report.txt` next to an unrelated `report.part` overwrote it, and
            // a destination that itself ends in `.part` staged onto itself.
            // A dropped NamedTempFile deletes itself, so a failed write leaves
            // nothing behind and never touches `local_path`.
            let dir = local_path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            let mut temp = tempfile::NamedTempFile::new_in(dir)?;
            temp.write_all(&bytes)?;
            temp.persist(local_path).map_err(|e| e.error)?;
            Ok(())
        })
    }

    /// Upload a local file to S3
    pub fn upload_object(&self, bucket: &str, key: &str, local_path: &Path) -> io::Result<()> {
        let body = std::fs::read(local_path)?;

        self.runtime.block_on(async {
            self.client
                .put_object()
                .bucket(bucket)
                .key(key)
                .body(body.into())
                .send()
                .await
                .map_err(|e| io::Error::other(e.to_string()))?;

            Ok(())
        })
    }

    /// Delete an object from S3
    pub fn delete_object(&self, bucket: &str, key: &str) -> io::Result<()> {
        self.runtime.block_on(async {
            self.client
                .delete_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| io::Error::other(e.to_string()))?;

            Ok(())
        })
    }

    /// Copy an object within S3
    pub fn copy_object(
        &self,
        source_bucket: &str,
        source_key: &str,
        dest_bucket: &str,
        dest_key: &str,
    ) -> io::Result<()> {
        self.runtime.block_on(async {
            let copy_source = format!("{source_bucket}/{source_key}");

            self.client
                .copy_object()
                .bucket(dest_bucket)
                .key(dest_key)
                .copy_source(&copy_source)
                .send()
                .await
                .map_err(|e| io::Error::other(e.to_string()))?;

            Ok(())
        })
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_objects_and_buckets;
#[cfg(test)]
mod tests_virtual_path;

async fn configured_client(profile_name: &str) -> Client {
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");

    let credentials_provider = ProfileFileCredentialsProvider::builder()
        .profile_name(profile_name)
        .build();

    let config = aws_config::defaults(BehaviorVersion::latest())
        .credentials_provider(credentials_provider)
        .region(region_provider)
        .load()
        .await;

    Client::new(&config)
}
