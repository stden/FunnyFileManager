//! Purpose: One machine as Things lists it, and the few values the dialog and
//! the import read from it.

use serde::Deserialize;

/// The state a machine is in. A state this program does not know reads as
/// [`Status::Unknown`], so a new one does not fail the whole list.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Online,
    Offline,
    Maintenance,
    #[default]
    #[serde(other)]
    Unknown,
}

impl Status {
    /// The state as Things spells it.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::Offline => "offline",
            Self::Maintenance => "maintenance",
            Self::Unknown => "unknown",
        }
    }
}

/// The operating system a machine runs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct OsInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
}

/// A machine in the catalog. A field the list leaves out reads as empty, so one
/// incomplete machine does not fail the whole list.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Machine {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub hostname: String,
    #[serde(default)]
    pub ip_address: Option<String>,
    #[serde(default)]
    pub status: Status,
    #[serde(default)]
    pub os_info: Option<OsInfo>,
    #[serde(default)]
    pub environment: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Machine {
    /// The address a session connects to: the hostname, or the IP address when
    /// the hostname is blank. Empty when neither is given.
    #[must_use]
    pub fn host(&self) -> &str {
        if self.hostname.trim().is_empty() {
            self.ip_address.as_deref().unwrap_or_default()
        } else {
            &self.hostname
        }
    }

    /// The operating system as `name version`, or empty when none is given.
    #[must_use]
    pub fn os(&self) -> String {
        self.os_info.as_ref().map_or_else(String::new, |os| {
            format!("{} {}", os.name, os.version).trim().to_owned()
        })
    }
}
