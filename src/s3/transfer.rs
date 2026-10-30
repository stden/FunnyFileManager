//! Purpose: `S3Client` object transfers: download, upload, delete and copy of one key.

use super::S3Client;
use std::io;
use std::path::Path;

const MAX_S3_OBJECT_BYTES: u64 = 100 * 1024 * 1024;

impl S3Client {
    /// Download an object to a local path
    pub fn download_object(&self, bucket: &str, key: &str, local_path: &Path) -> io::Result<()> {
        let target = crate::file_write::real_target(local_path)
            .map_err(crate::file_write::DanglingLink::into_io_error)?;
        crate::file_write::ensure_writable(&target)?;
        self.runtime.block_on(async {
            let resp = self
                .client
                .get_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| io::Error::other(e.to_string()))?;

            if resp
                .content_length()
                .is_some_and(|len| u64::try_from(len).unwrap_or(u64::MAX) > MAX_S3_OBJECT_BYTES)
            {
                return Err(io::Error::other("S3 object exceeds 100 MiB download limit"));
            }

            let data = resp
                .body
                .collect()
                .await
                .map_err(|e| io::Error::other(e.to_string()))?;

            let bytes = data.into_bytes();
            if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_S3_OBJECT_BYTES {
                return Err(io::Error::other("S3 object exceeds 100 MiB download limit"));
            }
            persist_download(local_path, &bytes)
        })
    }

    /// Upload a local file to S3
    pub fn upload_object(&self, bucket: &str, key: &str, local_path: &Path) -> io::Result<()> {
        let meta = std::fs::symlink_metadata(local_path)?;
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Err(io::Error::other("Refusing to upload non-regular file"));
        }
        if meta.len() > MAX_S3_OBJECT_BYTES {
            return Err(io::Error::other("File exceeds 100 MiB S3 upload limit"));
        }
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
        if source_bucket.is_empty()
            || source_key.is_empty()
            || source_bucket
                .chars()
                .any(|c| matches!(c, '?' | '#' | '/' | '%') || c.is_control())
            || source_key
                .chars()
                .any(|c| matches!(c, '?' | '#' | '%') || c.is_control())
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid S3 copy source",
            ));
        }
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

fn persist_download(local_path: &Path, bytes: &[u8]) -> io::Result<()> {
    let target = crate::file_write::real_target(local_path)
        .map_err(crate::file_write::DanglingLink::into_io_error)?;
    crate::file_write::ensure_writable(&target)?;
    let temp = crate::file_write::temp_beside(&target)?;
    crate::file_write::persist_over(temp, &target, bytes)
}
