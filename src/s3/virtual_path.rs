//! Purpose: `VirtualPath`, the union of a local path and an `s3://` location, with its parser and display form.

use std::path::PathBuf;

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
        let Some(rest) = path.strip_prefix("s3://") else {
            return Self::Local(PathBuf::from(path));
        };
        let (bucket_part, prefix) = rest.split_once('/').unwrap_or((rest, ""));
        if bucket_part.is_empty() {
            return Self::S3 {
                profile: "default".to_owned(),
                bucket: None,
                prefix: String::new(),
            };
        }

        // Format: s3://bucket/prefix or s3://profile@bucket/prefix.
        let (profile, bucket) = bucket_part
            .split_once('@')
            .unwrap_or(("default", bucket_part));
        Self::S3 {
            profile: profile.to_owned(),
            bucket: (!bucket.is_empty()).then(|| bucket.to_owned()),
            prefix: prefix.to_owned(),
        }
    }

    /// Convert to display string
    pub fn display(&self) -> String {
        match self {
            Self::Local(p) => p.display().to_string(),
            Self::S3 {
                profile,
                bucket: None,
                ..
            } => format!("s3://{profile}@"),
            Self::S3 {
                profile,
                bucket: Some(bucket),
                prefix,
            } if prefix.is_empty() => format!("s3://{profile}@{bucket}"),
            Self::S3 {
                profile,
                bucket: Some(bucket),
                prefix,
            } => format!("s3://{profile}@{bucket}/{prefix}"),
        }
    }
}
