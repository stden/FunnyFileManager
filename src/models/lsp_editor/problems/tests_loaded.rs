//! Purpose: A problem that is not a duplicate was never a duplicate the baseline
//! had loaded.

use super::*;

#[test]
fn an_invalid_extension_is_never_a_loaded_duplicate() {
    let invalid = Problem::InvalidExtension {
        language: LanguageId::from_key("python"),
        extension: ".".to_owned(),
    };
    let baseline = LspSettings(BTreeMap::new());
    assert!(!duplicate_was_loaded(&invalid, &baseline));
}
