//! Purpose: `S3Client` object transfers: download, upload, delete and copy of one key.

use super::S3Client;
use std::io;
use std::io::Write as _;
use std::path::Path;

impl S3Client {
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
