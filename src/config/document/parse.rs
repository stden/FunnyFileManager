//! Purpose: Parsing a stored settings file — YAML or legacy JSON — into the
//! one top-level mapping, refusing the YAML tags no setting uses.

use serde::de::DeserializeOwned;
use serde_norway::{Mapping, Value};

use super::Format;

/// `document` as `T`; the error names the setting that failed by its path.
pub(super) fn parse_mapping<T: DeserializeOwned>(
    document: &Mapping,
) -> Result<T, serde_path_to_error::Error<serde_norway::Error>> {
    serde_path_to_error::deserialize(Value::Mapping(document.clone()))
}

/// The mapping of settings `text` holds. An empty or comment-only file holds
/// none; a legacy file's pages move under `options`.
pub(super) fn parse_document(text: &str, format: Format) -> Result<Mapping, String> {
    let value: Value = match format {
        Format::Yaml => serde_norway::from_str(text).map_err(|error| error.to_string())?,
        Format::LegacyJson => serde_json::from_str(text).map_err(|error| error.to_string())?,
    };
    let document = match value {
        Value::Mapping(document) => document,
        Value::Null => Mapping::new(),
        _ => return Err("expected a mapping of settings".to_owned()),
    };
    if let Some(tag) = tag_in_entries(&document) {
        return Err(format!(
            "{tag} is a YAML tag, which no setting uses; \
             put a value that begins with ! in single quotes"
        ));
    }
    Ok(match format {
        Format::Yaml => document,
        Format::LegacyJson => crate::models::settings::nest_legacy_pages(document),
    })
}

/// The first YAML tag among `entries`, after the path to it:
/// `options.Masks: !*.tmp`.
fn tag_in_entries(entries: &Mapping) -> Option<String> {
    entries
        .iter()
        .find_map(|(key, value)| Some(format!("{}{}", key.as_str().unwrap_or("?"), tag_in(value)?)))
}

/// The first YAML tag in `value`, after the rest of the path to it.
fn tag_in(value: &Value) -> Option<String> {
    match value {
        Value::Tagged(tagged) => Some(format!(": {}", tagged.tag)),
        Value::Mapping(entries) => tag_in_entries(entries).map(|found| format!(".{found}")),
        Value::Sequence(items) => items
            .iter()
            .enumerate()
            .find_map(|(index, item)| Some(format!("[{index}]{}", tag_in(item)?))),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => None,
    }
}
