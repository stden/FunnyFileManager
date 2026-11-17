//! Purpose: An invalid extension is a problem of its server, but it is not a
//! conflict between two servers, so the detail pane does not list it as one.

use std::collections::BTreeMap;

use super::*;
use crate::config::LspSettings;
use crate::models::lsp_editor::Access;

#[test]
fn an_invalid_extension_is_not_listed_as_a_conflict() {
    let server = ServerSettings {
        extensions: vec![".py".to_owned()],
        ..ServerSettings::default()
    };
    let loaded = LspSettings(BTreeMap::from([("python".to_owned(), server)]));
    let editor = LspEditor::new(
        Access::Writable {
            path: "/srv/x/settings.yaml".into(),
        },
        loaded,
    );
    let python = LanguageId::from_key("python");
    assert_eq!(conflicts(&editor, &python), Vec::<String>::new());
}
