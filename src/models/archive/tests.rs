//! Unit tests: the Add to archive dialog's state — the tab ring, the format
//! radio renaming the archive as it turns, and the name offered on opening.

use std::path::Path;

use super::{ArchiveField, ArchiveFormat, ArchivePrompt};

fn names(sources: &[&str]) -> Vec<String> {
    sources.iter().map(|name| (*name).to_owned()).collect()
}

fn prompt(sources: &[&str]) -> ArchivePrompt {
    let sources = names(sources);
    let stem = ArchivePrompt::suggest_stem(&sources, false, "work");
    ArchivePrompt::new(
        sources,
        &stem,
        Path::new("/other"),
        vec![ArchiveFormat::TarGz, ArchiveFormat::Tar],
    )
}

/// One file is offered as `<stem>.tar.gz` in the other panel, so the
/// commonest case — pack this one thing over there — is one Enter away.
#[test]
fn a_single_file_is_offered_as_its_stem_in_the_other_panel() {
    let prompt = prompt(&["notes.txt"]);
    assert_eq!(
        Path::new(prompt.name.text()),
        Path::new("/other/notes.tar.gz")
    );
    assert_eq!(prompt.format, ArchiveFormat::TarGz);
    assert!(!prompt.delete_after);
    assert_eq!(prompt.field, ArchiveField::Name);
}

/// A directory keeps its whole name: `v1.2` is not `v1`. A dotfile has no
/// extension to strip either.
#[test]
fn a_directory_and_a_dotfile_keep_their_whole_name() {
    assert_eq!(
        ArchivePrompt::suggest_stem(&names(&["v1.2"]), true, "work"),
        "v1.2"
    );
    assert_eq!(
        ArchivePrompt::suggest_stem(&names(&[".bashrc"]), false, "work"),
        ".bashrc"
    );
    assert_eq!(
        ArchivePrompt::suggest_stem(&names(&["notes.txt"]), false, "work"),
        "notes"
    );
}

/// Several files are named after the folder they are in, as Far's archiver
/// plugin does: it is the one name that describes all of them.
#[test]
fn several_files_are_named_after_their_folder() {
    let prompt = prompt(&["a.txt", "b.txt"]);
    assert_eq!(
        Path::new(prompt.name.text()),
        Path::new("/other/work.tar.gz")
    );
    assert_eq!(prompt.caption(), "Add 2 items to the archive:");
}

#[test]
fn the_caption_names_a_single_source() {
    assert_eq!(
        prompt(&["notes.txt"]).caption(),
        "Add \"notes.txt\" to the archive:"
    );
}

/// Tab walks Name → Format → Delete → Add → Cancel and wraps; Shift+Tab walks
/// it back.
#[test]
fn the_tab_ring_visits_every_control_and_wraps() {
    let mut prompt = prompt(&["notes.txt"]);
    let mut seen = vec![prompt.field];
    for _ in 0..4 {
        prompt.advance(true);
        seen.push(prompt.field);
    }
    assert_eq!(
        seen,
        [
            ArchiveField::Name,
            ArchiveField::Format,
            ArchiveField::DeleteAfter,
            ArchiveField::Add,
            ArchiveField::Cancel
        ]
    );
    prompt.advance(true);
    assert_eq!(prompt.field, ArchiveField::Name, "wraps forward");
    prompt.advance(false);
    assert_eq!(prompt.field, ArchiveField::Cancel, "wraps back");
}

/// Turning the radio renames the archive to match: the name and the format
/// must agree, and making the user retype the extension is how they stop
/// agreeing.
#[test]
fn turning_the_format_radio_renames_the_extension() {
    let mut prompt = prompt(&["notes.txt"]);
    prompt.field = ArchiveField::Format;
    prompt.cycle(true);
    assert_eq!(prompt.format, ArchiveFormat::Tar);
    assert_eq!(Path::new(prompt.name.text()), Path::new("/other/notes.tar"));
    assert_eq!(prompt.format_index(), 1);
    prompt.cycle(true);
    assert_eq!(prompt.format, ArchiveFormat::TarGz, "only offered formats");
    assert_eq!(
        Path::new(prompt.name.text()),
        Path::new("/other/notes.tar.gz")
    );
    prompt.cycle(false);
    assert_eq!(prompt.format, ArchiveFormat::Tar);
}

/// A name the user has already changed is left alone by the radio: the
/// extension is swapped only when it is the one the old format gave.
#[test]
fn a_hand_typed_name_is_not_renamed_by_the_radio() {
    let mut prompt = prompt(&["notes.txt"]);
    prompt.name.set_text("/other/custom");
    prompt.field = ArchiveField::Format;
    prompt.cycle(true);
    assert_eq!(prompt.name.text(), "/other/custom");
}

/// The arrows belong to the radio only: on the name field they must not turn
/// it, or typing a path would silently change the format.
#[test]
fn the_arrows_do_nothing_off_the_radio() {
    let mut prompt = prompt(&["notes.txt"]);
    prompt.cycle(true);
    assert_eq!(prompt.format, ArchiveFormat::TarGz);
}

#[test]
fn space_toggles_only_the_checkbox() {
    let mut prompt = prompt(&["notes.txt"]);
    prompt.toggle();
    assert!(!prompt.delete_after, "not on the checkbox");
    prompt.field = ArchiveField::DeleteAfter;
    assert!(prompt.field.is_checkbox());
    prompt.toggle();
    assert!(prompt.delete_after);
}

#[test]
fn an_empty_name_is_not_ready() {
    let mut prompt = prompt(&["notes.txt"]);
    assert!(prompt.is_ready());
    prompt.name.set_text("  ");
    assert!(!prompt.is_ready());
}

#[test]
fn every_format_knows_its_extension_and_tool() {
    assert_eq!(ArchiveFormat::TarGz.extension(), "tar.gz");
    assert_eq!(ArchiveFormat::Tar.extension(), "tar");
    assert_eq!(ArchiveFormat::Zip.extension(), "zip");
    assert_eq!(ArchiveFormat::Zip.tool(), "zip");
    assert_eq!(ArchiveFormat::Tar.tool(), "tar");
    assert_eq!(ArchiveFormat::ALL.len(), 3);
}

/// An existing archive's name says its format; only a plain tar and a zip
/// can be added to, since a compressed tar is one stream.
#[test]
fn an_existing_archive_names_its_format_and_whether_it_can_grow() {
    assert_eq!(
        ArchiveFormat::from_name("a.tar.gz"),
        Some(ArchiveFormat::TarGz)
    );
    assert_eq!(ArchiveFormat::from_name("A.TAR"), Some(ArchiveFormat::Tar));
    assert_eq!(ArchiveFormat::from_name("a.zip"), Some(ArchiveFormat::Zip));
    assert_eq!(ArchiveFormat::from_name("a.tar.xz"), None);
    assert_eq!(ArchiveFormat::from_name("notes.txt"), None);
    assert!(ArchiveFormat::Tar.appendable());
    assert!(ArchiveFormat::Zip.appendable());
    assert!(!ArchiveFormat::TarGz.appendable());
}

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
