//! Purpose: Declarative language-server commands, as the `lsp` key of the
//! settings file stores them.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path, sync::OnceLock};

mod install;
pub mod patch;
pub mod values;
pub use install::{InstallCommand, InstallRecipe};
use values::Enablement;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerSettings {
    pub enabled: Enablement,
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
            enabled: Enablement::Enabled,
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
    fn default() -> Self {
        Self::catalog().clone()
    }
}

/// Each user entry is merged onto the catalog entry of the same language field
/// by field, so a partial override keeps the catalog's other fields. An error
/// names the entry that failed (`python: …`); the reader's own path already
/// names the `lsp` key above it.
impl<'de> Deserialize<'de> for LspSettings {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let overrides = BTreeMap::<String, serde_json::Value>::deserialize(deserializer)?;
        let mut settings = Self::default();
        for (language, value) in overrides {
            let server = merge(settings.0.get(&language), value)
                .map_err(|error| serde::de::Error::custom(format_args!("{language}: {error}")))?;
            settings.0.insert(language, server);
        }
        Ok(settings)
    }
}

/// `overrides` laid over `base`, or over the default server for a language
/// the catalog does not know.
fn merge(
    base: Option<&ServerSettings>,
    overrides: serde_json::Value,
) -> Result<ServerSettings, serde_json::Error> {
    let serde_json::Value::Object(fields) = overrides else {
        return Err(serde::de::Error::custom("LSP settings must be an object"));
    };
    let mut merged = serde_json::to_value(base.cloned().unwrap_or_default())?;
    merged
        .as_object_mut()
        .ok_or_else(|| serde::de::Error::custom("LSP server defaults must be an object"))?
        .extend(fields);
    serde_json::from_value(merged)
}

impl LspSettings {
    /// The embedded server catalog, parsed once for the life of the process.
    ///
    /// # Panics
    ///
    /// Never at run time: the catalog is compiled in, and every configuration
    /// test parses it.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "the catalog is compiled in and every configuration test parses it"
    )]
    pub fn catalog() -> &'static Self {
        static CATALOG: OnceLock<LspSettings> = OnceLock::new();
        CATALOG.get_or_init(|| {
            Self(
                serde_json::from_str(include_str!("lsp/catalog.json"))
                    .expect("the embedded server catalog is validated by configuration tests"),
            )
        })
    }

    pub(crate) fn for_path(&self, path: &Path) -> Option<(&str, &ServerSettings)> {
        let extension = path.extension()?.to_str()?;
        self.0
            .iter()
            .find(|(_, server)| {
                server.enabled == Enablement::Enabled
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
