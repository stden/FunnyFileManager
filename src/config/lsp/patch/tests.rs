//! Purpose: Pin the smallest-change rule: only changed owned fields are
//! written, a catalog value is unset rather than pinned, every other key
//! keeps its value and its place, and a save is refused when `lsp` changed
//! on disk. The diff is pinned in `tests/changes.rs`, entries that are no
//! mapping in `tests/entries.rs`, and the saves in `tests/saves.rs`.

use std::{fs, io, path::Path};

use serde_norway::Mapping;
use tempfile::TempDir;

use super::save::lsp_mapping;
use super::*;
use crate::config::{Enablement, InstallRecipe, LspSettings, Settings};

mod changes;
mod entries;
mod saves;

/// `text` as YAML: a mapping, or a scalar.
fn yaml<T: serde::de::DeserializeOwned>(text: &str) -> T {
    serde_norway::from_str(text).expect("valid YAML")
}

/// The table loaded from `lsp` as the file holds it, and that raw mapping.
fn loaded(lsp: &str) -> (LspSettings, Mapping) {
    (yaml(lsp), yaml(lsp))
}

/// `lsp` after the draft that `edit` makes of it is saved.
fn saved(lsp: &str, edit: impl FnOnce(&mut LspSettings)) -> Mapping {
    let (baseline, mut raw) = loaded(lsp);
    let mut draft = baseline.clone();
    edit(&mut draft);
    apply(&diff(&baseline, &draft), &mut raw).expect("applies");
    raw
}

fn server<'a>(settings: &'a mut LspSettings, language: &str) -> &'a mut ServerSettings {
    settings.0.get_mut(language).expect("a server")
}

fn python() -> LanguageId {
    LanguageId::from_key("python")
}

/// The top-level keys of `mapping`, in order.
fn keys(mapping: &Mapping) -> Vec<&str> {
    mapping.keys().filter_map(Value::as_str).collect()
}

/// The four keys cannot drift from `ServerSettings`' serde names.
#[test]
fn every_owned_key_is_the_serde_name_of_its_field() {
    let server = ServerSettings {
        install: Some(InstallRecipe::Python {
            package: "p".to_owned(),
            module: "m".to_owned(),
        }),
        ..ServerSettings::default()
    };
    let value = serde_norway::to_value(&server).expect("serialize");
    for field in OwnedField::ALL {
        assert_eq!(
            value[field.key()],
            field.value_of(&server),
            "{}",
            field.key()
        );
    }
}

/// The four owned fields come from `from`; the recipe and the root markers
/// stay the server's own.
#[test]
fn with_owned_fields_copies_the_owned_four_and_keeps_the_rest() {
    let python = LspSettings::catalog().0["python"].clone();
    let from = ServerSettings {
        enabled: Enablement::Disabled,
        command: "p".to_owned(),
        args: vec!["-v".to_owned()],
        extensions: vec!["pp".to_owned()],
        root_markers: vec!["elsewhere".to_owned()],
        install: None,
    };
    let merged = with_owned_fields(python.clone(), &from);
    for field in OwnedField::ALL {
        assert_eq!(field.value_of(&merged), field.value_of(&from), "{field:?}");
    }
    assert_eq!(merged.root_markers, python.root_markers);
    assert_eq!(merged.install, python.install);
}
