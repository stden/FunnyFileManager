//! Purpose: Where a session came from, how that is stored, and how the forms
//! that earlier builds wrote are still read.
//!
//! Stored as a mapping with a `kind`: `kind: netbox_file` with its `path`, or
//! `kind: things` with its `machine`. A manual origin, the default, is not
//! stored at all. Earlier builds wrote the same variants with serde's default
//! enum encoding: `Manual` as a plain string, and the other two as a
//! single-key mapping in `sessions.json` (`{"NetBoxFile": path}`) or as YAML
//! tags in `ffm.yml` (`!NetBoxFile path`, and `!Things` over a `machine`
//! mapping). [`read`] takes the current mapping and those earlier shapes, and
//! [`untag_legacy_origins`] rewrites those two tags into the mapping form; any
//! other tag stays, and the settings reader refuses it.

use std::path::PathBuf;

use serde::{Deserialize, Deserializer, Serialize};
use serde_norway::{Mapping, Value};

/// Where a session came from, so an import can tell an entry the user made or
/// changed — which it must keep — from one it wrote itself and may refresh.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Origin {
    /// Made or last changed in the session dialog.
    #[default]
    Manual,
    /// Read from this `.netbox` file and not changed since.
    #[serde(rename = "netbox_file")]
    NetBoxFile { path: PathBuf },
    /// Made from the Things catalog's machine with this id and not changed since.
    Things { machine: String },
}

impl Origin {
    #[must_use]
    pub const fn is_manual(&self) -> bool {
        matches!(self, Self::Manual)
    }
}

/// Either shape an origin may be stored in.
#[derive(Deserialize)]
#[serde(untagged)]
enum Stored {
    Current(Origin),
    Earlier(Earlier),
}

/// The variants as serde's default encoding wrote them before they had a `kind`.
#[derive(Deserialize)]
enum Earlier {
    Manual,
    NetBoxFile(PathBuf),
    Things { machine: String },
}

impl From<Earlier> for Origin {
    fn from(earlier: Earlier) -> Self {
        match earlier {
            Earlier::Manual => Self::Manual,
            Earlier::NetBoxFile(path) => Self::NetBoxFile { path },
            Earlier::Things { machine } => Self::Things { machine },
        }
    }
}

/// An origin in whichever shape it was stored in; the session field's
/// `deserialize_with`.
pub(super) fn read<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Origin, D::Error> {
    Ok(match Stored::deserialize(deserializer)? {
        Stored::Current(origin) => origin,
        Stored::Earlier(earlier) => earlier.into(),
    })
}

/// Rewrites the origin of each session that holds `!NetBoxFile` or `!Things`
/// into the mapping [`read`] takes: `!NetBoxFile path` becomes
/// `{NetBoxFile: path}`. Any other tag is left in place, so the settings reader
/// refuses it by name.
pub fn untag_legacy_origins(document: &mut Mapping) {
    let Some(Value::Sequence(sessions)) = document.get_mut("sessions") else {
        return;
    };
    for session in sessions {
        let Some(origin) = session.get_mut("origin") else {
            continue;
        };
        let Value::Tagged(tagged) = origin else {
            continue;
        };
        let tag = tagged.tag.to_string();
        if !matches!(tag.as_str(), "!NetBoxFile" | "!Things") {
            continue;
        }
        let entry = [(
            Value::String(tag.trim_start_matches('!').to_owned()),
            tagged.value.clone(),
        )];
        *origin = Value::Mapping(entry.into_iter().collect());
    }
}
