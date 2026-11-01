//! Purpose: A relative vault path is shown under the working folder, so the
//! title names the full file.

use std::path::Path;

use super::*;

#[test]
fn a_relative_vault_path_is_shown_under_the_working_folder() {
    let shown = display_path(Path::new("vault.kdbx"));
    assert!(shown.ends_with("vault.kdbx"), "{shown}");
    assert!(shown.len() > "vault.kdbx".len(), "{shown}");
}
