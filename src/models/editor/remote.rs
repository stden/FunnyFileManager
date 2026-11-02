//! Purpose: A remote editor retains its temporary backing file and the version read from the host.

use super::Fingerprint;
use crate::models::remote_work::FileLocation;
use std::sync::Arc;

#[derive(Clone)]
pub struct RemoteDocument {
    pub location: FileLocation,
    pub fingerprint: Fingerprint,
    pub temporary: Arc<tempfile::TempDir>,
}
