//! Purpose: Declarative language-server commands using the existing JSON settings format.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerSettings {
    pub enabled: bool,
    pub command: String,
    pub args: Vec<String>,
    pub extensions: Vec<String>,
    pub root_markers: Vec<String>,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            command: "rust-analyzer".into(),
            args: Vec::new(),
            extensions: vec!["rs".into()],
            root_markers: vec!["Cargo.toml".into(), ".git".into()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LspSettings(pub BTreeMap<String, ServerSettings>);

impl Default for LspSettings {
    fn default() -> Self {
        Self(BTreeMap::from([("rust".into(), ServerSettings::default())]))
    }
}

impl LspSettings {
    pub(crate) fn for_path(&self, path: &Path) -> Option<(&str, &ServerSettings)> {
        let extension = path.extension()?.to_str()?;
        self.0
            .iter()
            .find(|(_, server)| {
                server.enabled && server.extensions.iter().any(|ext| ext == extension)
            })
            .map(|(language, server)| (language.as_str(), server))
    }
}
