//! Purpose: A server nothing has probed yet shows a blank tag, as wide as a
//! real one, rather than a made-up availability.

use std::collections::BTreeMap;

use super::*;
use crate::config::LspSettings;
use crate::models::lsp_editor::Access;

#[test]
fn a_server_with_no_availability_yet_shows_a_blank_tag() {
    let editor = LspEditor::new(
        Access::Writable {
            path: "/srv/x/settings.yaml".into(),
        },
        LspSettings(BTreeMap::new()),
    );
    let language = LanguageId::from_key("python");
    assert_eq!(tag(&editor, &language), (" ".repeat(TAG), Style::default()));
}
