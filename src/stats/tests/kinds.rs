//! Purpose: Content kinds — how a file is categorised, and the breakdown by kind.

use super::*;

#[test]
fn a_file_is_categorised_by_its_extension() {
    assert_eq!(ContentKind::of("main.rs"), ContentKind::Source);
    assert_eq!(ContentKind::of("notes.md"), ContentKind::Document);
    assert_eq!(ContentKind::of("photo.JPG"), ContentKind::Image);
    assert_eq!(ContentKind::of("song.flac"), ContentKind::Audio);
    assert_eq!(ContentKind::of("clip.mkv"), ContentKind::Video);
    assert_eq!(ContentKind::of("bundle.tar.gz"), ContentKind::Archive);
    assert_eq!(ContentKind::of("libthing.so"), ContentKind::Executable);
    assert_eq!(ContentKind::of("data.json"), ContentKind::Data);
}

/// No extension, or one nobody has heard of, is `Other` — not a guess.
#[test]
fn an_unknown_or_absent_extension_is_other() {
    assert_eq!(ContentKind::of("Makefile"), ContentKind::Other);
    assert_eq!(ContentKind::of("archive.zzzz"), ContentKind::Other);
    assert_eq!(ContentKind::of(""), ContentKind::Other);
    // A leading dot is a hidden file, not an extension.
    assert_eq!(ContentKind::of(".gitignore"), ContentKind::Other);
}

#[test]
fn the_breakdown_totals_each_category_and_orders_by_size() {
    let dir = tree(&[
        ("a.rs", 100),
        ("b.rs", 200),
        ("doc.md", 50),
        ("media/clip.mp4", 9000),
    ]);
    let stats = analyse(dir.path());

    assert_eq!(kind(&stats, ContentKind::Source).unwrap().size.bytes(), 300);
    assert_eq!(kind(&stats, ContentKind::Source).unwrap().files, 2);
    assert_eq!(
        kind(&stats, ContentKind::Document).unwrap().size.bytes(),
        50
    );
    assert_eq!(
        stats.by_kind.first().map(|k| k.kind),
        Some(ContentKind::Video),
        "the largest category leads: {:?}",
        stats.by_kind
    );
}

/// Every byte lands in exactly one category, so the breakdown adds up to the
/// total — otherwise the view quietly loses files.
#[test]
fn the_categories_account_for_every_byte() {
    let dir = tree(&[
        ("a.rs", 100),
        ("b.png", 250),
        ("c.unknown", 7),
        ("sub/d.zip", 1000),
    ]);
    let stats = analyse(dir.path());

    let summed: u64 = stats.by_kind.iter().map(|k| k.size.bytes()).sum();
    assert_eq!(summed, stats.total.bytes());
    let counted: usize = stats.by_kind.iter().map(|k| k.files).sum();
    assert_eq!(counted, stats.files);
}

/// Every category prints a name. A missing or duplicated one would put two
/// different kinds of file on the same row of the panel.
#[test]
fn every_content_kind_has_its_own_label() {
    use super::ContentKind;

    let kinds = [
        ContentKind::Source,
        ContentKind::Document,
        ContentKind::Image,
        ContentKind::Audio,
        ContentKind::Video,
        ContentKind::Archive,
        ContentKind::Executable,
        ContentKind::Data,
        ContentKind::Other,
    ];
    let mut labels: Vec<&str> = kinds.iter().map(|kind| kind.label()).collect();
    assert!(labels.iter().all(|label| !label.is_empty()));
    labels.sort_unstable();
    let count = labels.len();
    labels.dedup();
    assert_eq!(labels.len(), count, "two kinds share a label");
}
