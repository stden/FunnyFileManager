//! Language-server settings use real state with empty and overflowing details.

use std::collections::BTreeMap;

use crate::{
    config::{Enablement, LspSettings, ServerSettings},
    models::{lsp_editor::Access, LspEditor, Overlay},
};

use super::{overlay, Case};

pub(super) fn cases() -> Vec<Case> {
    let mut empty = overlay(
        "language-servers-empty-values",
        Overlay::LspEditor(Box::new(editor(String::new()))),
        &["Language servers", "Settings:"],
    );
    empty.wide_required = vec!["Arguments:", "Auto-install:", "(none)", "off"];
    let mut long = overlay(
        "language-servers-long-detail",
        Overlay::LspEditor(Box::new(editor("language-server-".repeat(200)))),
        &["Language servers", "Settings:"],
    );
    long.wide_required = vec!["F3 shows everything"];
    vec![empty, long, switched_off()]
}

fn switched_off() -> Case {
    let mut settings = BTreeMap::new();
    for language in ["python", "rust"] {
        settings.insert(
            language.to_owned(),
            ServerSettings {
                enabled: Enablement::Disabled,
                command: format!("{language}-server"),
                extensions: vec![language.to_owned()],
                ..ServerSettings::default()
            },
        );
    }
    overlay(
        "language-servers-switched-off",
        Overlay::LspEditor(Box::new(LspEditor::new(access(), LspSettings(settings)))),
        &["Language servers", "rust"],
    )
}

fn editor(command: String) -> LspEditor {
    LspEditor::new(
        access(),
        LspSettings(BTreeMap::from([(
            "rust".to_owned(),
            ServerSettings {
                command,
                ..ServerSettings::default()
            },
        )])),
    )
}

fn access() -> Access {
    Access::Writable {
        path: "/ffm-quality/settings.yaml".into(),
    }
}
