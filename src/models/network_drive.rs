//! Purpose: Validated local mounts and remote addresses with explicit network protocols.
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkProtocol {
    Smb,
    Nfs,
    Sshfs,
    WebDav,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct MountPoint(String);

impl TryFrom<String> for MountPoint {
    type Error = NetworkDriveError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let windows = value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic)
            && value.as_bytes().get(1..3) == Some(b":\\");
        if value.chars().any(char::is_control)
            || !(Path::new(&value).is_absolute() || value.starts_with('/') || windows)
        {
            return Err(NetworkDriveError::InvalidMount);
        }
        Ok(Self(value))
    }
}
impl From<MountPoint> for String {
    fn from(value: MountPoint) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct NetworkAddress(String);
impl TryFrom<String> for NetworkAddress {
    type Error = NetworkDriveError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() || value.chars().any(char::is_control) {
            return Err(NetworkDriveError::InvalidAddress);
        }
        if has_credentials(&value) {
            return Err(NetworkDriveError::EmbeddedCredentials);
        }
        Ok(Self(value))
    }
}
impl From<NetworkAddress> for String {
    fn from(value: NetworkAddress) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkDrive {
    pub path: MountPoint,
    pub remote: NetworkAddress,
    pub protocol: NetworkProtocol,
}

fn has_credentials(value: &str) -> bool {
    if let Some((_, address)) = value.split_once("://") {
        let authority = address.split('/').next().unwrap_or_default();
        return authority.contains('@') || address.contains(['?', '#']);
    }
    value
        .split_once('@')
        .is_some_and(|(user, _)| user.contains(':'))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkDriveError {
    InvalidMount,
    InvalidAddress,
    EmbeddedCredentials,
}
impl std::fmt::Display for NetworkDriveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidMount => "mount point must be an absolute path without control characters",
            Self::InvalidAddress => "network address is empty or contains control characters",
            Self::EmbeddedCredentials => "network addresses must not contain embedded credentials",
        })
    }
}
impl std::error::Error for NetworkDriveError {}
