//! Purpose: Pin what the Windows PATH store makes of the registry's `Path`.

use super::*;

fn value(kind: RegistryStringKind, text: &str) -> RegistryString {
    RegistryString {
        kind,
        text: text.to_owned(),
    }
}

#[test]
fn a_missing_path_reads_as_empty_and_expandable() {
    let missing = stored(None);
    assert_eq!(missing, value(RegistryStringKind::Expandable, ""));
    let present = value(RegistryStringKind::Plain, r"C:\Tools");
    assert_eq!(stored(Some(present.clone())), present);
}

/// A save refuses when either the text or the kind changed since the read.
#[test]
fn the_revision_changes_with_the_text_and_with_the_kind() {
    let read = value(RegistryStringKind::Expandable, r"%USERPROFILE%\bin");
    let same = value(RegistryStringKind::Expandable, r"%USERPROFILE%\bin");
    let retyped = value(RegistryStringKind::Plain, r"%USERPROFILE%\bin");
    let edited = value(
        RegistryStringKind::Expandable,
        r"%USERPROFILE%\bin;C:\Tools",
    );
    assert_eq!(revision(&read), revision(&same));
    assert_ne!(revision(&read), revision(&retyped));
    assert_ne!(revision(&read), revision(&edited));
}

#[test]
fn a_new_value_stays_expandable_or_becomes_it_for_a_variable() {
    let plain = value(RegistryStringKind::Plain, r"C:\Tools");
    let expandable = value(RegistryStringKind::Expandable, r"C:\Tools");
    assert_eq!(
        replacement(&plain, r"C:\Tools;C:\Go\bin"),
        value(RegistryStringKind::Plain, r"C:\Tools;C:\Go\bin")
    );
    assert_eq!(
        replacement(&plain, r"C:\Tools;%GOPATH%\bin").kind,
        RegistryStringKind::Expandable
    );
    assert_eq!(
        replacement(&expandable, r"C:\Tools").kind,
        RegistryStringKind::Expandable
    );
}
