//! Purpose: Server profiles in the user's ffm.yml, preserving other YAML sections.
use super::{Backing, SessionStore, StoreError, FILE_NAME, MAX_FILE_BYTES};
use crate::models::Session;
use std::{io, path::Path};

pub(super) const USER_FILE_NAME: &str = "ffm.yml";

#[derive(serde::Serialize, serde::Deserialize)]
struct Profiles {
    #[serde(default)]
    sessions: Vec<Session>,
}

pub(super) fn load(path: &Path) -> SessionStore {
    match read(path) {
        Ok(sessions) => SessionStore {
            sessions: sessions.clone(),
            backing: Backing::Yaml {
                path: path.to_owned(),
                loaded: sessions,
            },
        },
        Err(error) => SessionStore {
            sessions: Vec::new(),
            backing: Backing::Damaged(format!("{USER_FILE_NAME}: {error}")),
        },
    }
}

fn read(path: &Path) -> io::Result<Vec<Session>> {
    match super::read_text(path, MAX_FILE_BYTES) {
        Ok(_) => {
            let file: Profiles = crate::config::document::read(path)?
                .ok_or_else(|| io::Error::other("configuration disappeared"))?
                .parse()?;
            validate(&file.sessions)?;
            Ok(file.sessions)
        }
        Err(super::ReadError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
            super::file::read_sessions(&path.with_file_name(FILE_NAME))
                .map_err(|error| io::Error::other(error.to_string()))
        }
        Err(error) => Err(io::Error::other(error.to_string())),
    }
}

fn validate(sessions: &[Session]) -> io::Result<()> {
    let names: std::collections::BTreeSet<_> =
        sessions.iter().map(|session| &session.name).collect();
    if names.len() != sessions.len() {
        return Err(io::Error::other("two sessions share a name"));
    }
    Ok(())
}

pub(super) fn save(
    path: &Path,
    loaded: &[Session],
    sessions: &[Session],
) -> Result<(), StoreError> {
    let current = Profiles {
        sessions: sessions.to_owned(),
    };
    let text = serde_norway::to_string(&current)
        .map_err(|error| StoreError::Write(io::Error::other(error)))?;
    if u64::try_from(text.len()).unwrap_or(u64::MAX) > MAX_FILE_BYTES {
        return Err(StoreError::TooLarge {
            limit: MAX_FILE_BYTES,
        });
    }
    crate::config::document::update(
        path,
        &Profiles {
            sessions: loaded.to_owned(),
        },
        &current,
    )
    .map_err(StoreError::Write)
}
