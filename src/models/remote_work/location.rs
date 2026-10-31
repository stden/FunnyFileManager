//! Purpose: Checked local and remote locations for file operations.

use crate::models::RemotePath;
use crate::remote::{RemoteError, RemoteLink};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileLocation {
    Local(PathBuf),
    Remote { link: RemoteLink, path: RemotePath },
}

impl FileLocation {
    pub(crate) fn parent(&self) -> Result<Self, RemoteError> {
        let error = RemoteError::InvalidInput("a filesystem root has no parent");
        match self {
            Self::Local(path) => path
                .parent()
                .map(|parent| Self::Local(parent.to_path_buf()))
                .ok_or(error),
            Self::Remote { link, path } => path
                .parent()
                .map(|path| Self::Remote {
                    link: link.clone(),
                    path,
                })
                .ok_or(error),
        }
    }

    pub(crate) fn resolve(&self, text: &str) -> Result<Self, RemoteError> {
        if text.is_empty() {
            return Err(RemoteError::InvalidInput("a destination is required"));
        }
        match self {
            Self::Local(base) => Ok(Self::Local(if Path::new(text).is_absolute() {
                PathBuf::from(text)
            } else {
                base.join(text)
            })),
            Self::Remote { link, path } => {
                let text = if text.starts_with('/') {
                    text.to_owned()
                } else {
                    format!("{path}/{text}")
                };
                let path = RemotePath::parse(&text).map_err(|_protocol_error| {
                    RemoteError::InvalidInput("invalid remote destination")
                })?;
                Ok(Self::Remote {
                    link: link.clone(),
                    path,
                })
            }
        }
    }

    pub(crate) fn child(&self, name: &str) -> Result<Self, RemoteError> {
        let invalid =
            || RemoteError::InvalidInput("file name cannot be represented at the destination");
        match self {
            Self::Local(path) => {
                let mut parts = Path::new(name).components();
                if !matches!(parts.next(), Some(Component::Normal(_))) || parts.next().is_some() {
                    return Err(invalid());
                }
                Ok(Self::Local(path.join(name)))
            }
            Self::Remote { link, path } => Ok(Self::Remote {
                link: link.clone(),
                path: path.join(name).ok_or_else(invalid)?,
            }),
        }
    }

    pub(crate) fn name(&self) -> Result<&str, RemoteError> {
        match self {
            Self::Local(path) => path.file_name().and_then(|name| name.to_str()),
            Self::Remote { path, .. } => path
                .as_str()
                .rsplit('/')
                .next()
                .filter(|name| !name.is_empty()),
        }
        .ok_or(RemoteError::InvalidInput(
            "cannot transfer a root or non-UTF-8 name",
        ))
    }

    pub(crate) fn label(&self) -> String {
        match self {
            Self::Local(path) => path.display().to_string(),
            Self::Remote { path, .. } => path.to_string(),
        }
    }
}
