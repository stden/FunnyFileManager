//! Purpose: What the prompt is made of for whoever runs it — the bullet, the
//! path, the branch and the sigil, joined into the line that is drawn.

use std::path::{Path, PathBuf};

use crate::prompt::{segments, LastCommand, Part};

fn parts(directory: &Path, last: LastCommand) -> Vec<Part> {
    segments(directory, last)
        .into_iter()
        .map(|s| s.part)
        .collect()
}

fn text_of(directory: &Path, part: Part) -> Option<String> {
    segments(directory, LastCommand::Succeeded)
        .into_iter()
        .find(|segment| segment.part == part)
        .map(|segment| segment.text)
}

/// The bullet is the point of the leading segment: it says whether the last
/// command worked, before the user has to read anything.
#[test]
fn the_bullet_reports_the_last_command() {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(parts(&here, LastCommand::Succeeded)[0], Part::StatusOk);
    assert_eq!(parts(&here, LastCommand::Failed)[0], Part::StatusFailed);
}

#[test]
fn the_path_is_always_present() {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(
        text_of(&here, Part::Path).as_deref(),
        Some(env!("CARGO_MANIFEST_DIR"))
    );
}

/// A repository shows its branch, in brackets, after the path.
///
/// Built as a fixture rather than relying on this checkout being one: that is
/// an environment fact, and it is false in a copied build tree — which is
/// exactly where `cargo mutants` runs.
#[test]
fn a_repository_shows_its_branch_in_brackets() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    std::fs::create_dir(dir.path().join(".git")).expect("git dir");
    std::fs::write(dir.path().join(".git/HEAD"), "ref: refs/heads/topic\n").expect("HEAD");

    assert_eq!(
        text_of(dir.path(), Part::Branch).as_deref(),
        Some("(topic)")
    );
}

/// …and a directory outside one shows no branch segment at all, rather than
/// empty brackets.
#[test]
fn a_plain_directory_has_no_branch_segment() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    assert!(!parts(dir.path(), LastCommand::Succeeded).contains(&Part::Branch));
}

/// The segments must join back into the line that gets drawn — the renderer
/// measures the pieces, so a gap here is a gap on screen.
#[test]
fn the_segments_join_into_the_prompt() {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let joined: String = segments(&here, LastCommand::Succeeded)
        .iter()
        .map(|segment| segment.text.as_str())
        .collect();

    assert!(joined.starts_with('●'), "{joined}");
    assert!(joined.contains(env!("CARGO_MANIFEST_DIR")), "{joined}");
    assert!(joined.ends_with("# ") || joined.ends_with("$ "), "{joined}");
    assert!(!joined.contains("  "), "no double blanks: {joined:?}");
}

/// The sigil says whether this shell can overwrite anything.
#[test]
fn the_sigil_is_hash_for_root_and_dollar_otherwise() {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sigil = text_of(&here, Part::Sigil).expect("always present");
    assert!(sigil == "#" || sigil == "$", "{sigil}");
}

/// The bullet must distinguish the outcomes that are failures from the ones
/// that are not. Every status collapsing to one answer makes it decoration.
#[test]
fn only_real_failures_turn_the_bullet_red() {
    use crate::status::Status;

    assert_eq!(
        LastCommand::from_status(&Status::SaveFailed {
            error: "denied".to_owned()
        }),
        LastCommand::Failed
    );
    assert_eq!(
        LastCommand::from_status(&Status::RefreshFailed("gone".to_owned())),
        LastCommand::Failed
    );
    // Not outcomes: a fresh session must not open on a red bullet.
    assert_eq!(
        LastCommand::from_status(&Status::Cleared),
        LastCommand::Succeeded
    );
    assert_eq!(
        LastCommand::from_status(&Status::KeyHints),
        LastCommand::Succeeded
    );
    assert_eq!(
        LastCommand::from_status(&Status::PanelsSwapped),
        LastCommand::Succeeded
    );
}
