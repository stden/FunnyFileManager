//! Purpose: Appending to an existing archive names that archive and keeps its format.

use std::path::Path;

use super::{names, prompt, ArchiveField, ArchiveFormat, ArchivePrompt};

/// The Archive commands row opens the dialog on the archive itself, with a
/// radio that cannot turn: the format is the one the archive already has.
#[test]
fn appending_names_the_archive_and_fixes_its_format() {
    let mut appending = ArchivePrompt::appending(
        names(&["a.txt"]),
        Path::new("/other/old.tar"),
        ArchiveFormat::Tar,
    );
    assert!(appending.append);
    assert_eq!(appending.name.text(), "/other/old.tar");
    assert_eq!(appending.offered, [ArchiveFormat::Tar]);
    appending.field = ArchiveField::Format;
    appending.cycle(true);
    assert_eq!(appending.format, ArchiveFormat::Tar);
    assert_eq!(appending.name.text(), "/other/old.tar");
    assert!(!prompt(&["a.txt"]).append, "Shift+F1 makes a new archive");
}
