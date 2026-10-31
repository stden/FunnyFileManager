//! Purpose: Bound remote text downloads even if a file grows while it is read.

use crate::models::remote_work::FileLocation;
use crate::remote::RemoteError;
use std::io::Write;

pub(super) fn bytes(source: &FileLocation) -> Result<Vec<u8>, RemoteError> {
    let limit = crate::utils::MAX_FILE_SIZE.bytes();
    let mut buffer = Bounded {
        bytes: Vec::new(),
        limit,
    };
    match source {
        FileLocation::Remote { link, path } => {
            link.lock().download(path, &mut buffer)?;
        }
        FileLocation::Local(path) => {
            std::io::copy(&mut crate::utils::open_regular_file(path)?, &mut buffer)?;
        }
    }
    Ok(buffer.bytes)
}

struct Bounded {
    bytes: Vec<u8>,
    limit: u64,
}

impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let length =
            u64::try_from(self.bytes.len().saturating_add(bytes.len())).unwrap_or(u64::MAX);
        if length > self.limit {
            return Err(std::io::Error::other(
                "remote file exceeds the viewer/editor size limit",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
