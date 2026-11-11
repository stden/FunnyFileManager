//! Purpose: Merge discovered mounts under the user's YAML lock, preserving offline mounts.
use crate::models::network_drive::NetworkDrive;
use std::{collections::BTreeMap, io, path::Path};

#[derive(serde::Deserialize)]
struct Document {
    #[serde(default)]
    network_drives: Vec<NetworkDrive>,
}
pub fn save_to(path: &Path, drives: &[NetworkDrive]) -> io::Result<()> {
    if drives.is_empty() {
        return Ok(());
    }
    crate::config::document::edit(path, |document, stored: &Document| {
        migrate_sessions(document, path)?;
        let mounts: BTreeMap<_, _> = stored
            .network_drives
            .iter()
            .chain(drives)
            .map(|drive| (drive.path.clone(), drive.clone()))
            .collect();
        let values: Vec<_> = mounts.into_values().collect();
        document.insert(
            "network_drives".into(),
            serde_norway::to_value(values).map_err(io::Error::other)?,
        );
        Ok(())
    })
    .map(|_: Document| ())
}

fn migrate_sessions(document: &mut serde_norway::Mapping, path: &Path) -> io::Result<()> {
    let legacy = path.with_file_name(crate::sessions::FILE_NAME);
    if document.contains_key("sessions") || !legacy.exists() {
        return Ok(());
    }
    let store = crate::sessions::SessionStore::load_from(legacy);
    if store.is_damaged() {
        return Err(io::Error::other(
            "legacy server profiles could not be migrated",
        ));
    }
    document.insert(
        "sessions".into(),
        serde_norway::to_value(store.sorted()).map_err(io::Error::other)?,
    );
    Ok(())
}
