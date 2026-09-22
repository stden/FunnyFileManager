//! Purpose: the rows an S3 listing yields (buckets, folders, files) and their conversion from SDK types.

use crate::models::{EntryKind, FileSize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
    // `checked_add`: a corrupt stamp far past what `SystemTime` can hold is
    // the same kind of nonsense as a negative one, and must not panic.
    UNIX_EPOCH.checked_add(Duration::from_secs(secs))
}

/// Convert a common prefix into the directory row relative to this listing.
pub(super) fn folder_from_prefix(key: &str, prefix: &str) -> Option<S3Object> {
    let name = key
        .strip_prefix(prefix)
        .unwrap_or(key)
        .trim_end_matches('/');
    if name.is_empty() {
        return None;
    }
    Some(S3Object {
        key: name.to_string(),
        size: FileSize::from_bytes(0),
        kind: EntryKind::Directory,
        last_modified: None,
    })
}

/// Convert object metadata into a file row, excluding the current folder marker.
pub(super) fn file_from_object(
    object: &aws_sdk_s3::types::Object,
    prefix: &str,
) -> Option<S3Object> {
    let key = object.key()?;
    if key == prefix {
        return None;
    }
    let name = key.strip_prefix(prefix).unwrap_or(key);
    if name.is_empty() || name == "/" {
        return None;
    }
    Some(S3Object {
        key: name.to_string(),
        size: FileSize::from_bytes(object.size().unwrap_or(0).max(0) as u64),
        kind: EntryKind::File,
        last_modified: object.last_modified().and_then(instant_of),
    })
}
