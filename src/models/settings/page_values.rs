//! Purpose: The Options pages' values as the settings file stores them — one
//! key per page, named after it, under `options` (`options: {Editor: '8'}`).
//!
//! The file also holds `config::Settings`, whose keys (`lsp`, `system`,
//! `editor`, …) are its own. The pages live under one key of their own so that
//! no page sits beside a typed key that differs from it only in case
//! (`Editor` and `editor`). Builds before YAML wrote the pages flat, beside
//! the typed keys, in `settings.json`; [`nest_legacy_pages`] moves them.

use std::collections::BTreeMap;

use serde::de::{self, Deserialize, Deserializer, IntoDeserializer};
use serde::Serialize;
use serde_norway::{Mapping, Value};

use crate::menu::SettingsPage;

/// The key the Options pages live under, as [`OptionsSection`] spells it.
const SECTION_KEY: &str = "options";

/// Each page's text, as the Options dialog edits it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct PageValues(pub BTreeMap<SettingsPage, String>);

/// The Options pages as they sit in the settings file: `options: {…}`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(default)]
pub struct OptionsSection {
    pub options: PageValues,
}

impl<'de> Deserialize<'de> for PageValues {
    /// Keys that name no page are skipped: a newer build may add pages.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Mapping::deserialize(deserializer)?
            .into_iter()
            .filter_map(|(key, value)| {
                let name = key.as_str()?.to_owned();
                Some((page_named(&name)?, name, value))
            })
            .map(|(page, name, value)| {
                page_text(value)
                    .map(|text| (page, text))
                    .map_err(|error| de::Error::custom(format!("{name}: {error}")))
            })
            .collect::<Result<_, _>>()
            .map(Self)
    }
}

/// A page's value as the text the Options dialog holds.
///
/// A person writing YAML leaves numbers and switches unquoted, so any scalar
/// is text: `Editor: 8` is `8`, and an empty value or `~` is empty. The text
/// is what YAML resolved, so it is also what the file says after any save
/// rewrites it: `0x10` is `16` before a save as well as after.
fn page_text(value: Value) -> Result<String, &'static str> {
    match value {
        Value::String(text) => Ok(text.trim().to_owned()),
        Value::Number(number) => Ok(number.to_string()),
        Value::Bool(flag) => Ok(flag.to_string()),
        Value::Null => Ok(String::new()),
        Value::Sequence(_) | Value::Mapping(_) | Value::Tagged(_) => {
            Err("expected text, not a list, a mapping or a tagged value")
        }
    }
}

/// The page a settings key names, spelled as a save writes it.
fn page_named(key: &str) -> Option<SettingsPage> {
    let key: de::value::StrDeserializer<'_, de::value::Error> = key.into_deserializer();
    SettingsPage::deserialize(key).ok()
}

/// A legacy `settings.json` in the layout `settings.yaml` uses: the keys that
/// name a page move, in order, under `options`; every other key stays.
#[must_use]
pub fn nest_legacy_pages(legacy: Mapping) -> Mapping {
    let (pages, mut document): (Mapping, Mapping) = legacy
        .into_iter()
        .partition(|(key, _)| key.as_str().and_then(page_named).is_some());
    if !pages.is_empty() {
        document.insert(Value::from(SECTION_KEY), Value::Mapping(pages));
    }
    document
}
