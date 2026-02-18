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
                            size: 0,
                            is_folder: true,
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
    use aws_sdk_s3::config::{Credentials, Region, SharedCredentialsProvider};
    use aws_sdk_s3::Client;
    use aws_smithy_http_client::test_util::{ReplayEvent, StaticReplayClient};
    use aws_smithy_types::body::SdkBody;
    use http_1x::{Request, Response};
    use std::env;
    use std::io;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn build_test_client(events: Vec<ReplayEvent>) -> S3Client {
        let http_client = StaticReplayClient::new(events);
        let config = aws_sdk_s3::Config::builder()
            .credentials_provider(SharedCredentialsProvider::new(Credentials::for_tests()))
            .region(Region::new("us-east-1"))
            .http_client(http_client)
            .behavior_version_latest()
            .with_test_defaults()
            .build();
        let client = Client::from_conf(config);
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        S3Client {
            client,
            runtime,
            profile: "test".to_string(),
            current_bucket: None,
            current_prefix: String::new(),
        }
    }

    fn replay_event(xml_body: &str) -> ReplayEvent {
        ReplayEvent::new(
            Request::builder()
                .uri("https://s3.us-east-1.amazonaws.com/")
                .body(SdkBody::empty())
                .expect("request"),
            Response::builder()
                .status(200)
                .body(SdkBody::from(xml_body))
                .expect("response"),
        )
    }

    fn ok_event() -> ReplayEvent {
        ReplayEvent::new(
            Request::builder()
                .uri("https://s3.us-east-1.amazonaws.com/")
                .body(SdkBody::empty())
                .expect("request"),
            Response::builder()
                .status(200)
                .body(SdkBody::empty())
                .expect("response"),
        )
    }

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
    fn test_virtual_path_display_formats() {
        let local = VirtualPath::Local(PathBuf::from("/tmp"));
        assert_eq!(local.display(), "/tmp");

        let s3_root = VirtualPath::S3 {
            profile: "default".to_string(),
            bucket: None,
            prefix: String::new(),
        };
        assert_eq!(s3_root.display(), "s3://default");

        let s3_bucket = VirtualPath::S3 {
            profile: "prod".to_string(),
            bucket: Some("logs".to_string()),
            prefix: String::new(),
        };
        assert_eq!(s3_bucket.display(), "s3://prod@logs");

        let s3_prefix = VirtualPath::S3 {
            profile: "prod".to_string(),
            bucket: Some("logs".to_string()),
            prefix: "2024/".to_string(),
        };
        assert_eq!(s3_prefix.display(), "s3://prod@logs/2024/");
    }

    #[test]
    fn test_list_aws_profiles() {
        // This test just ensures the function doesn't panic
        let profiles = list_aws_profiles();
        // Should at least have "default"
        assert!(profiles.iter().any(|p| p.name == "default"));
    }

    #[test]
    fn test_list_aws_profiles_reads_credentials() {
        let temp_dir = TempDir::new().expect("temp dir");
        let credentials_dir = temp_dir.path().join(".aws");
        std::fs::create_dir_all(&credentials_dir).expect("create .aws");
        std::fs::write(
            credentials_dir.join("credentials"),
            "[default]\naws_access_key_id=AKIA\n[team]\naws_secret_access_key=SECRET\n",
        )
        .expect("write credentials");

        let original_home = env::var("HOME").ok();
        env::set_var("HOME", temp_dir.path());

        let profiles = list_aws_profiles();
        assert!(profiles.iter().any(|p| p.name == "default"));
        assert!(profiles.iter().any(|p| p.name == "team"));

        if let Some(home) = original_home {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }
    }

    #[test]
    fn test_s3_client_list_buckets_and_objects() -> io::Result<()> {
        let list_buckets_xml = r#"
            <ListAllMyBucketsResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
                <Buckets>
                    <Bucket>
                        <Name>alpha</Name>
                        <CreationDate>2020-01-01T00:00:00.000Z</CreationDate>
                    </Bucket>
                </Buckets>
            </ListAllMyBucketsResult>
        "#;
        let list_objects_xml = r#"
            <ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
                <Name>alpha</Name>
                <Prefix>folder/</Prefix>
                <Delimiter>/</Delimiter>
                <CommonPrefixes>
                    <Prefix>folder/subdir/</Prefix>
                </CommonPrefixes>
                <Contents>
                    <Key>folder/file.txt</Key>
                    <LastModified>2020-01-01T00:00:00.000Z</LastModified>
                    <Size>123</Size>
                </Contents>
            </ListBucketResult>
        "#;
        let client = build_test_client(vec![
            replay_event(list_buckets_xml),
            replay_event(list_objects_xml),
        ]);

        let buckets = client.list_buckets()?;
        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].name, "alpha");
        assert!(buckets[0].creation_date.is_some());

        let objects = client.list_objects("alpha", "folder/")?;
        assert!(objects.iter().any(|obj| obj.is_folder && obj.key == "subdir"));
        assert!(objects.iter().any(|obj| !obj.is_folder && obj.key == "file.txt"));
        assert!(objects.iter().any(|obj| obj.last_modified.is_some()));

        Ok(())
    }

    #[test]
    fn test_s3_client_object_operations() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("temp dir");
        let download_path = temp_dir.path().join("download.txt");
        let upload_path = temp_dir.path().join("upload.txt");
        std::fs::write(&upload_path, "payload")?;

        let get_response = ReplayEvent::new(
            Request::builder()
                .uri("https://s3.us-east-1.amazonaws.com/alpha/download.txt")
                .body(SdkBody::empty())
                .expect("request"),
            Response::builder()
                .status(200)
                .body(SdkBody::from("downloaded"))
                .expect("response"),
        );
        let client = build_test_client(vec![
            get_response,
            ok_event(),
            ok_event(),
            ok_event(),
        ]);

        client.download_object("alpha", "download.txt", &download_path)?;
        assert_eq!(std::fs::read_to_string(&download_path)?, "downloaded");

        client.upload_object("alpha", "upload.txt", &upload_path)?;
        client.copy_object("alpha", "upload.txt", "alpha", "copied.txt")?;
        client.delete_object("alpha", "copied.txt")?;

        Ok(())
    }
}
