//! AWS S3 integration module for FunnyFileManager
//!
//! Provides S3 bucket browsing as virtual drives.

use aws_config::meta::region::RegionProviderChain;
use aws_config::profile::ProfileFileCredentialsProvider;
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;
use std::io;
use std::path::PathBuf;

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
    pub size: u64,
    pub is_folder: bool,
    pub last_modified: Option<String>,
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
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let client = runtime.block_on(async {
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
        });

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
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

            let buckets = resp
                .buckets()
                .iter()
                .map(|b| S3Bucket {
                    name: b.name().unwrap_or_default().to_string(),
                    creation_date: b.creation_date().map(|d| d.to_string()),
                })
                .collect();

            Ok(buckets)
        })
    }

    /// List objects in a bucket with optional prefix (folder path)
    pub fn list_objects(&self, bucket: &str, prefix: &str) -> io::Result<Vec<S3Object>> {
        self.runtime.block_on(async {
            let mut request = self
                .client
                .list_objects_v2()
                .bucket(bucket)
                .delimiter("/");

            if !prefix.is_empty() {
                request = request.prefix(prefix);
            }

            let resp = request
                .send()
                .await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

            let mut objects = Vec::new();

            // Add "folders" (common prefixes)
            if let Some(prefixes) = resp.common_prefixes() {
                for p in prefixes {
                    if let Some(prefix_str) = p.prefix() {
                        // Remove the current prefix to get the folder name
                        let folder_name = prefix_str
                            .strip_prefix(prefix)
                            .unwrap_or(prefix_str)
                            .trim_end_matches('/');

                        if !folder_name.is_empty() {
                            objects.push(S3Object {
                                key: folder_name.to_string(),
                                size: 0,
                                is_folder: true,
                                last_modified: None,
                            });
                        }
                    }
                }
            }

            // Add files
            if let Some(contents) = resp.contents() {
                for obj in contents {
                    if let Some(key) = obj.key() {
                        // Skip the prefix itself if it appears as an object
                        if key == prefix {
                            continue;
                        }

                        let name = key
                            .strip_prefix(prefix)
                            .unwrap_or(key)
                            .to_string();

                        // Skip if this is just a folder marker
                        if name.is_empty() || name == "/" {
                            continue;
                        }

                        objects.push(S3Object {
                            key: name,
                            size: obj.size().unwrap_or(0) as u64,
                            is_folder: false,
                            last_modified: obj.last_modified().map(|d| d.to_string()),
                        });
                    }
                }
            }

            Ok(objects)
        })
    }

    /// Download an object to a local path
    pub fn download_object(
        &self,
        bucket: &str,
        key: &str,
        local_path: &PathBuf,
    ) -> io::Result<()> {
        self.runtime.block_on(async {
            let resp = self
                .client
                .get_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

            let data = resp
                .body
                .collect()
                .await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

            std::fs::write(local_path, data.into_bytes())?;
            Ok(())
        })
    }

    /// Upload a local file to S3
    pub fn upload_object(
        &self,
        bucket: &str,
        key: &str,
        local_path: &PathBuf,
    ) -> io::Result<()> {
        let body = std::fs::read(local_path)?;

        self.runtime.block_on(async {
            self.client
                .put_object()
                .bucket(bucket)
                .key(key)
                .body(body.into())
                .send()
                .await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

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
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

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
            let copy_source = format!("{}/{}", source_bucket, source_key);

            self.client
                .copy_object()
                .bucket(dest_bucket)
                .key(dest_key)
                .copy_source(&copy_source)
                .send()
                .await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

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
                if line.starts_with('[') && line.ends_with(']') {
                    let name = line[1..line.len() - 1].to_string();
                    profiles.push(AwsProfile {
                        name,
                        region: None,
                    });
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
#[derive(Clone, Debug, PartialEq)]
pub enum VirtualPath {
    Local(PathBuf),
    S3 {
        profile: String,
        bucket: Option<String>,
        prefix: String,
    },
}

impl VirtualPath {
    /// Parse a path string into a VirtualPath
    pub fn parse(path: &str) -> Self {
        if path.starts_with("s3://") {
            let rest = &path[5..];
            let parts: Vec<&str> = rest.splitn(2, '/').collect();

            if parts.is_empty() || parts[0].is_empty() {
                VirtualPath::S3 {
                    profile: "default".to_string(),
                    bucket: None,
                    prefix: String::new(),
                }
            } else {
                // Format: s3://bucket/prefix or s3://profile@bucket/prefix
                let bucket_part = parts[0];
                let prefix = parts.get(1).unwrap_or(&"").to_string();

                if bucket_part.contains('@') {
                    let bp: Vec<&str> = bucket_part.splitn(2, '@').collect();
                    VirtualPath::S3 {
                        profile: bp[0].to_string(),
                        bucket: Some(bp[1].to_string()),
                        prefix,
                    }
                } else {
                    VirtualPath::S3 {
                        profile: "default".to_string(),
                        bucket: Some(bucket_part.to_string()),
                        prefix,
                    }
                }
            }
        } else {
            VirtualPath::Local(PathBuf::from(path))
        }
    }

    /// Convert to display string
    pub fn display(&self) -> String {
        match self {
            VirtualPath::Local(p) => p.display().to_string(),
            VirtualPath::S3 {
                profile,
                bucket,
                prefix,
            } => {
                if let Some(b) = bucket {
                    if prefix.is_empty() {
                        format!("s3://{}@{}", profile, b)
                    } else {
                        format!("s3://{}@{}/{}", profile, b, prefix)
                    }
                } else {
                    format!("s3://{}", profile)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_path_parse_local() {
        let path = VirtualPath::parse("/home/user/docs");
        assert!(matches!(path, VirtualPath::Local(_)));
    }

    #[test]
    fn test_virtual_path_parse_s3_bucket() {
        let path = VirtualPath::parse("s3://my-bucket/folder/file.txt");
        match path {
            VirtualPath::S3 {
                profile,
                bucket,
                prefix,
            } => {
                assert_eq!(profile, "default");
                assert_eq!(bucket, Some("my-bucket".to_string()));
                assert_eq!(prefix, "folder/file.txt");
            }
            _ => panic!("Expected S3 path"),
        }
    }

    #[test]
    fn test_virtual_path_parse_s3_with_profile() {
        let path = VirtualPath::parse("s3://myprofile@my-bucket/folder/");
        match path {
            VirtualPath::S3 {
                profile,
                bucket,
                prefix,
            } => {
                assert_eq!(profile, "myprofile");
                assert_eq!(bucket, Some("my-bucket".to_string()));
                assert_eq!(prefix, "folder/");
            }
            _ => panic!("Expected S3 path"),
        }
    }

    #[test]
    fn test_list_aws_profiles() {
        // This test just ensures the function doesn't panic
        let profiles = list_aws_profiles();
        // Should at least have "default"
        assert!(profiles.iter().any(|p| p.name == "default"));
    }
}
