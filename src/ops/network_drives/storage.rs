//! Purpose: Merge discovered mounts under the user's YAML lock, preserving offline mounts.
use crate::models::network_drive::NetworkDrive;
use std::{collections::BTreeMap, io, path::Path};

#[derive(serde::Deserialize)]
struct Document {
    #[serde(default)]
    network_drives: Vec<NetworkDrive>,
}

const TEMPLATE: &str = "# FunnyFileManager configuration (ffm.yml)\n\
# Network drives connected in one press via Alt+F1/F2, F9 Commands, or :netdrives\n\
# Examples:\n\
# network_drives:\n\
#   - path: 'Z:\\'\n\
#     remote: '\\\\example.test\\share'\n\
#     protocol: smb\n\
#   - path: /mnt/team\n\
#     remote: 'example.test:/export/team'\n\
#     protocol: nfs\n\
network_drives: []\n";

/// Load configured network drives from `path`, returning an empty list when `path` does not exist.
pub fn load_from(path: &Path) -> io::Result<Vec<NetworkDrive>> {
    let Some(stored) = crate::config::document::read_yaml(path)? else {
        return Ok(Vec::new());
    };
    let document: Document = stored.parse()?;
    Ok(document.network_drives)
}

/// Create `path` with a commented starter `network_drives:` section if it does not exist yet.
pub fn ensure_template(path: &Path) -> io::Result<()> {
    if path.exists() {
        return Ok(());
    }
    create_parent_dir(path)?;
    crate::file_write::replace_owner_only(
        path,
        "ffm.yml",
        TEMPLATE,
        crate::file_write::SyncPolicy::Always,
    )
}

pub(super) fn create_parent_dir(path: &Path) -> io::Result<()> {
    let Some(dir) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) else {
        return Ok(());
    };
    crate::file_write::create_dirs_owner_only(dir)
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
