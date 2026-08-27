//! AWS S3 integration module for `FunnyFileManager`
//!
//! Provides S3 bucket browsing as virtual drives.

use crate::models::{EntryKind, FileSize};
use aws_config::meta::region::RegionProviderChain;
use aws_config::profile::ProfileFileCredentialsProvider;
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;
use std::io;
use std::io::Write as _;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Represents an AWS profile from ~/.aws/credentials
#[derive(Clone, Debug)]
pub struct AwsProfile {
    pub name: String,
    pub region: Option<String>,
}

/// Represents an S3 bucket
#[derive(Clone, Debug)]
pub struct S3Bucket {
    pub name: String,
    pub creation_date: Option<String>,
}

/// Represents an S3 object (file or "folder")
#[derive(Clone, Debug)]
pub struct S3Object {
    pub key: String,
    pub size: FileSize,
    /// The same vocabulary the local listing uses, so the panel does not have
    /// to ask two different questions about the same row. S3 has no links, so
    /// only `Directory` and `File` ever appear here.
    pub kind: EntryKind,
    /// When the object was last written. A real instant rather than the
    /// SDK's rendered string: unparsed text cannot be sorted or compared, and
    /// sorting a listing by date is the first thing anyone asks of it.
    pub last_modified: Option<SystemTime>,
}

/// The SDK's timestamp as a `SystemTime`, or `None` for one before the epoch.
///
/// The SDK reports whole seconds plus nanos; only the seconds matter for a
/// listing, and a negative one is not a modification time any S3 object can
/// have — it is corrupt metadata, and dropping it is better than pretending.
fn instant_of(stamp: &aws_sdk_s3::primitives::DateTime) -> Option<SystemTime> {
    let secs = u64::try_from(stamp.secs()).ok()?;
    Some(UNIX_EPOCH + Duration::from_secs(secs))
}

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
            runtime.block_on(async {
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
            })
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

            let mut objects = Vec::new();

            // Add "folders" (common prefixes)
            for p in resp.common_prefixes() {
                if let Some(prefix_str) = p.prefix() {
                    // Remove the current prefix to get the folder name
                    let folder_name = prefix_str
                        .strip_prefix(prefix)
                        .unwrap_or(prefix_str)
                        .trim_end_matches('/');

                    if !folder_name.is_empty() {
                        objects.push(S3Object {
                            key: folder_name.to_string(),
                            size: FileSize::from_bytes(0),
                            kind: EntryKind::Directory,
                            last_modified: None,
                        });
                    }
                }
            }

            // Add files
            for obj in resp.contents() {
                if let Some(key) = obj.key() {
                    // Skip the prefix itself if it appears as an object
                    if key == prefix {
                        continue;
                    }

                    let name = key.strip_prefix(prefix).unwrap_or(key).to_string();

                    // Skip if this is just a folder marker
                    if name.is_empty() || name == "/" {
                        continue;
                    }

                    objects.push(S3Object {
                        key: name,
                        size: FileSize::from_bytes(obj.size().unwrap_or(0).max(0) as u64),
                        kind: EntryKind::File,
                        last_modified: obj.last_modified().and_then(instant_of),
                    });
                }
            }

            Ok(objects)
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

/// Get list of AWS profiles from ~/.aws/credentials
pub fn list_aws_profiles() -> Vec<AwsProfile> {
    let mut profiles = Vec::new();

    // Try to read credentials file
    if let Some(home) = dirs_next_candidate() {
        let credentials_path = home.join(".aws").join("credentials");
        if let Ok(content) = std::fs::read_to_string(&credentials_path) {
            for line in content.lines() {
                let line = line.trim();
                if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                    let name = name.to_string();
                    profiles.push(AwsProfile { name, region: None });
                }
            }
        }
    }

    // Always include "default" if not already present
    if !profiles.iter().any(|p| p.name == "default") {
        profiles.insert(
            0,
            AwsProfile {
                name: "default".to_string(),
                region: None,
            },
        );
    }

    profiles
}

/// Get home directory candidate
fn dirs_next_candidate() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}

/// Virtual path for S3 locations
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VirtualPath {
    Local(PathBuf),
    S3 {
        profile: String,
        bucket: Option<String>,
        prefix: String,
    },
}

impl VirtualPath {
    /// Parse a path string into a `VirtualPath`
    pub fn parse(path: &str) -> Self {
        if let Some(rest) = path.strip_prefix("s3://") {
            let (bucket_part, prefix_part) = rest.split_once('/').unwrap_or((rest, ""));

            if bucket_part.is_empty() {
                Self::S3 {
                    profile: "default".to_string(),
                    bucket: None,
                    prefix: String::new(),
                }
            } else {
                // Format: s3://bucket/prefix or s3://profile@bucket/prefix
                let prefix = prefix_part.to_string();

                if let Some((profile, bucket_name)) = bucket_part.split_once('@') {
                    Self::S3 {
                        profile: profile.to_string(),
                        bucket: if bucket_name.is_empty() {
                            None
                        } else {
                            Some(bucket_name.to_string())
                        },
                        prefix,
                    }
                } else {
                    Self::S3 {
                        profile: "default".to_string(),
                        bucket: Some(bucket_part.to_string()),
                        prefix,
                    }
                }
            }
        } else {
            Self::Local(PathBuf::from(path))
        }
    }

    /// Convert to display string
    pub fn display(&self) -> String {
        match self {
            Self::Local(p) => p.display().to_string(),
            Self::S3 {
                profile,
                bucket,
                prefix,
            } => {
                if let Some(b) = bucket {
                    if prefix.is_empty() {
                        format!("s3://{profile}@{b}")
                    } else {
                        format!("s3://{profile}@{b}/{prefix}")
                    }
                } else {
                    format!("s3://{profile}@")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_objects_and_buckets;
#[cfg(test)]
mod tests_virtual_path;
