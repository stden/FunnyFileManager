//! Purpose: Pin how changes meet an entry the file lacks or holds as something
//! other than a mapping: nothing to remove, or an error that writes nothing.

use super::*;

#[test]
fn an_entry_that_is_not_a_mapping_is_reported() {
    let mut raw: Mapping = yaml("python: 5");
    let set = [Change::Set {
        language: python(),
        field: OwnedField::Enabled,
        value: Value::Bool(false),
    }];
    let unset = [Change::Unset {
        language: python(),
        field: OwnedField::Enabled,
    }];
    for changes in [&set[..], &unset[..]] {
        let error = apply(changes, &mut raw).expect_err("not a mapping");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            error.to_string(),
            "lsp.python in settings.yaml is not a mapping"
        );
    }
    assert_eq!(raw["python"], Value::from(5), "left as it was");
}

/// Back at the catalog's value for a server the file has no entry for:
/// there is nothing to remove.
#[test]
fn an_unset_of_a_server_the_file_lacks_changes_nothing() {
    let mut raw = Mapping::new();
    let unset = [Change::Unset {
        language: python(),
        field: OwnedField::Command,
    }];
    apply(&unset, &mut raw).expect("nothing to do");
    assert_eq!(raw, Mapping::new());
}

/// A file whose `lsp` is neither a mapping nor empty does not parse, so a
/// save refuses before it gets here; were it reached, it still writes nothing.
#[test]
fn an_lsp_that_is_not_a_mapping_is_reported() {
    let mut document: Mapping = yaml("lsp: 5");
    let error = lsp_mapping(&mut document).expect_err("not a mapping");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(error.to_string(), "lsp in settings.yaml is not a mapping");
    let mut document = Mapping::new();
    assert_eq!(
        lsp_mapping(&mut document).map(|lsp| lsp.clone()).ok(),
        Some(Mapping::new()),
        "a file without `lsp` gets an empty one"
    );
}
