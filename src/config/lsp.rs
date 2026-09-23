//! Purpose: Declarative language-server commands, as the `lsp` key of the
//! settings file stores them.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

mod install;
pub use install::{InstallCommand, InstallRecipe};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerSettings {
    pub enabled: bool,
    pub command: String,
    pub args: Vec<String>,
    pub extensions: Vec<String>,
    pub root_markers: Vec<String>,
    /// None disables automatic installation for this server.
    pub install: Option<InstallRecipe>,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            command: "rust-analyzer".into(),
            args: Vec::new(),
            extensions: vec!["rs".into()],
            root_markers: vec!["Cargo.toml".into(), ".git".into()],
            install: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct LspSettings(pub BTreeMap<String, ServerSettings>);

impl Default for LspSettings {
    #[expect(
        clippy::expect_used,
        reason = "the catalog is compiled in and every configuration test parses it"
    )]
    fn default() -> Self {
        Self(
            serde_json::from_str(include_str!("lsp/catalog.json"))
                .expect("the embedded server catalog is validated by configuration tests"),
        )
    }
}

impl<'de> Deserialize<'de> for LspSettings {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let overrides = BTreeMap::<String, serde_json::Value>::deserialize(deserializer)?;
        let mut settings = Self::default();
        for (language, value) in overrides {
            let mut base =
                serde_json::to_value(settings.0.get(&language).cloned().unwrap_or_default())
                    .map_err(serde::de::Error::custom)?;
            let serde_json::Value::Object(fields) = value else {
                return Err(serde::de::Error::custom("LSP settings must be an object"));
            };
            base.as_object_mut()
                .ok_or_else(|| serde::de::Error::custom("LSP server defaults must be an object"))?
                .extend(fields);
            let server = serde_json::from_value(base).map_err(serde::de::Error::custom)?;
            settings.0.insert(language, server);
        }
        Ok(settings)
    }
}

impl LspSettings {
    pub(crate) fn for_path(&self, path: &Path) -> Option<(&str, &ServerSettings)> {
        let extension = path.extension()?.to_str()?;
        self.0
            .iter()
            .find(|(_, server)| {
                server.enabled
                    && server
                        .extensions
                        .iter()
                        .any(|ext| ext.eq_ignore_ascii_case(extension))
            })
            .map(|(language, server)| (language.as_str(), server))
    }
}

#[cfg(test)]
mod tests;
