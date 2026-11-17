//! Purpose: File URI validation and portable conversion without filesystem access.

use std::path::{Path, PathBuf};

pub fn from_path(path: &Path) -> Option<String> {
    url::Url::from_file_path(path).ok().map(Into::into)
}

pub fn to_path(uri: &str) -> Option<PathBuf> {
    let url = url::Url::parse(uri).ok()?;
    if url.scheme() != "file" || url.query().is_some() || url.fragment().is_some() {
        return None;
    }
    #[cfg(not(windows))]
    if url
        .host_str()
        .is_some_and(|h| !h.is_empty() && !h.eq_ignore_ascii_case("localhost"))
    {
        return None;
    }
    url.to_file_path()
        .ok()
        .filter(|path| !path.as_os_str().as_encoded_bytes().contains(&0))
}

pub fn same(left: &str, right: &str) -> bool {
    match (to_path(left), to_path(right)) {
        (Some(left), Some(right)) => same_path(&left, &right),
        _ => false,
    }
}

#[cfg(windows)]
pub fn same_path(left: &Path, right: &Path) -> bool {
    let left = crate::path_display::label(left).replace('/', "\\");
    let right = crate::path_display::label(right).replace('/', "\\");
    left.eq_ignore_ascii_case(&right)
}

#[cfg(not(windows))]
pub fn same_path(left: &Path, right: &Path) -> bool {
    left == right
}
