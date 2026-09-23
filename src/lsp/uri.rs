//! Purpose: File URI validation and portable conversion without filesystem access.

use std::path::{Path, PathBuf};

pub(crate) fn from_path(path: &Path) -> Option<String> {
    url::Url::from_file_path(path).ok().map(Into::into)
}

pub(crate) fn to_path(uri: &str) -> Option<PathBuf> {
    let url = url::Url::parse(uri).ok()?;
    if url.scheme() != "file" || url.query().is_some() || url.fragment().is_some() {
        return None;
    }
    url.to_file_path().ok()
}
