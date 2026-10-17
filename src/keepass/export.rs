//! Purpose: full-tree JSON/YAML export of a vault, secrets included.

use std::path::Path;

use keepass::db::{EntryRef, GroupRef, Times};
use serde::{Deserialize, Serialize};

use super::{open_db, KeepassError};

/// A complete, serializable snapshot of a kdbx database.
///
/// Contains the full group tree, every entry, and **every field — including
/// plaintext passwords**. The serialized output therefore holds secrets: write
/// it only to a protected location and delete it when done.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdbxExport {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub database_name: Option<String>,
    pub root: ExportGroup,
}

/// One group in the exported tree, with its child groups and entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportGroup {
    pub name: String,
    pub uuid: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub groups: Vec<Self>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub entries: Vec<ExportEntry>,
}

/// One entry in the exported tree. `fields` holds every string field (standard
/// and custom) with protected values decrypted to plaintext.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportEntry {
    pub uuid: String,
    pub fields: std::collections::BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub attachments: Vec<String>,
    #[serde(skip_serializing_if = "ExportTimes::is_empty", default)]
    pub times: ExportTimes,
}

/// Entry timestamps as strings (each omitted from the output when unset).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportTimes {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub modified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub expires: Option<String>,
}

impl ExportTimes {
    const fn is_empty(&self) -> bool {
        self.created.is_none() && self.modified.is_none() && self.expires.is_none()
    }
}

/// Extract the entire database into a [`KdbxExport`] tree.
///
/// Includes every group, every entry, all string fields (standard and custom)
/// **with plaintext passwords**, tags, attachment names, and timestamps.
pub fn export(
    db_path: &Path,
    password: &str,
    keyfile: Option<&Path>,
) -> Result<KdbxExport, KeepassError> {
    let db = open_db(db_path, password, keyfile)?;
    Ok(KdbxExport {
        database_name: db.meta.database_name.clone(),
        root: export_group(&db.root()),
    })
}

fn export_group(group: &GroupRef<'_>) -> ExportGroup {
    ExportGroup {
        name: group.name.clone(),
        uuid: group.id().uuid().to_string(),
        notes: group.notes.clone().filter(|n| !n.is_empty()),
        tags: group.tags.clone(),
        groups: group.groups().map(|g| export_group(&g)).collect(),
        entries: group.entries().map(|e| export_entry(&e)).collect(),
    }
}

fn export_entry(entry: &EntryRef<'_>) -> ExportEntry {
    // Collected into the sorted map, so the hash map's order never shows.
    let fields = entry
        .fields
        .keys()
        .filter_map(|key| entry.get(key).map(|value| (key.clone(), value.to_owned())))
        .collect();
    ExportEntry {
        uuid: entry.id().uuid().to_string(),
        fields,
        tags: entry.tags.clone(),
        attachments: entry
            .attachments_named()
            .map(|(name, _)| name.to_owned())
            .collect(),
        times: export_times(&entry.times),
    }
}

pub(super) fn export_times(times: &Times) -> ExportTimes {
    ExportTimes {
        created: times.creation.map(|t| t.to_string()),
        modified: times.last_modification.map(|t| t.to_string()),
        expires: if times.expires == Some(true) {
            times.expiry.map(|t| t.to_string())
        } else {
            None
        },
    }
}

/// Serialize a [`KdbxExport`] to pretty JSON. The output contains plaintext secrets.
pub fn to_json(export: &KdbxExport) -> Result<String, KeepassError> {
    serde_json::to_string_pretty(export).map_err(|e| KeepassError::Serialize(e.to_string()))
}

/// Serialize a [`KdbxExport`] to YAML. The output contains plaintext secrets.
pub fn to_yaml(export: &KdbxExport) -> Result<String, KeepassError> {
    serde_norway::to_string(export).map_err(|e| KeepassError::Serialize(e.to_string()))
}

/// Open a database and return its full contents as pretty JSON (contains secrets).
pub fn export_json(
    db_path: &Path,
    password: &str,
    keyfile: Option<&Path>,
) -> Result<String, KeepassError> {
    to_json(&export(db_path, password, keyfile)?)
}

/// Open a database and return its full contents as YAML (contains secrets).
pub fn export_yaml(
    db_path: &Path,
    password: &str,
    keyfile: Option<&Path>,
) -> Result<String, KeepassError> {
    to_yaml(&export(db_path, password, keyfile)?)
}
