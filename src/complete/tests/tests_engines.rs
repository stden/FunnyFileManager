//! Purpose: How the built-in scan and the fish engine combine for `cd`: failure modes, quoted tokens, dedup.

use super::*;

// ── `cd` path completion: the two engines and their failure modes ──────

/// The directory part names a regular file, so it cannot be listed. Completion
/// must offer nothing rather than falling back to the file's siblings.
#[test]
fn cd_completion_of_an_unlistable_directory_part_offers_nothing() {
    let tmp = empty_cwd();
    fs::write(tmp.path().join("notes.txt"), b"not a directory").expect("file");
    fs::create_dir(tmp.path().join("notes_dir")).expect("dir");
    fs::write(tmp.path().join("notes_dir").join("inner.txt"), b"x").expect("inner file");

    assert!(
        builtin_path_complete("notes.txt/", "i", tmp.path()).is_empty(),
        "a regular file must not be scanned as a directory"
    );
    assert!(
        complete("cd notes.txt/i", tmp.path()).is_none(),
        "a file used as a directory part must not complete"
    );

    // Control: the identical leaf completes when the directory part is real,
    // so the case above fails on the unlistable directory and nothing else.
    let real = complete("cd notes_dir/i", tmp.path()).expect("real directory completes");
    assert_eq!(real.line, "cd notes_dir/inner.txt ");
}

/// A quoted `cd` token: `"` belongs to no filename, so the built-in scan scores
/// nothing and any candidate can only have come from fish. This pins the merge
/// in `path_complete` — dropping it would leave quoted paths uncompletable on
/// the documented (fish-installed) setup.
#[test]
fn quoted_cd_token_completes_through_the_merged_fish_engine() {
    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("alpha")).expect("dir");

    assert!(
        builtin_path_complete("", "\"al", tmp.path()).is_empty(),
        "the quote must not match any filename"
    );

    // Serialize with the fake-PATH fish tests (they shadow `fish` on PATH).
    let _lock = crate::test_support::env_lock();
    let completed = complete("cd \"al", tmp.path());
    if fish_is_installed() {
        let completion = completed.expect("fish must supply the quoted-token candidate");
        assert_eq!(texts(&completion), vec!["alpha/".to_owned()]);
        assert_eq!(completion.line, "cd alpha/");
    } else {
        assert!(
            completed.is_none(),
            "without fish there is nothing to merge in"
        );
    }
}

/// A directory both engines return appears exactly once: `path_complete`
/// deduplicates the built-in scan against fish's candidates. Holds with fish
/// (both engines emit `alpha/`) and without it (only the built-in scan does).
#[test]
fn merged_completion_deduplicates_a_shared_directory() {
    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("alpha")).expect("dir");

    let completion = complete("cd al", tmp.path()).expect("alpha/ completes");
    let hits = completion
        .candidates
        .iter()
        .filter(|c| c.text == "alpha/")
        .count();
    assert_eq!(
        hits, 1,
        "shared entry must not be doubled: {:?}",
        completion.candidates
    );
}

// ── `cd` path completion: fish candidates merging into the built-in scan ───

/// fish's own candidates are merged into the built-in scan's pool: a shared
/// entry must not be doubled, and a fish-only entry must still show up.
/// A stub `fish` (via [`FISH_PROGRAM_FOR_TESTS`]) makes this
/// deterministic without a real fish install.
#[cfg(unix)]
#[test]
fn path_complete_merges_new_fish_candidates_and_skips_duplicates_of_the_builtin_scan() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("alpha")).expect("dir");

    let stub = tmp.path().join("stub-fish.sh");
    fs::write(
        &stub,
        "#!/bin/sh\nprintf 'alpha/\\tDirectory\\n'\nprintf 'beta/\\tDirectory\\n'\n",
    )
    .expect("write stub");
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).expect("chmod");
    wait_until_executable(&stub);

    FISH_PROGRAM_FOR_TESTS.with(|slot| *slot.borrow_mut() = Some(stub));
    let result = path_complete("al", tmp.path());
    FISH_PROGRAM_FOR_TESTS.with(|slot| *slot.borrow_mut() = None);

    assert_eq!(
        result.iter().filter(|c| *c == "alpha/").count(),
        1,
        "the entry both engines agree on must not be doubled: {result:?}"
    );
    assert!(
        result.contains(&"beta/".to_owned()),
        "a fish-only candidate must be merged in: {result:?}"
    );
}
