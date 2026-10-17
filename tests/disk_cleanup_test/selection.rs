//! Purpose: What a dialog selects: checked candidates, measured bytes and a fresh dialog.

use super::*;

/// Four rows covering every combination the two selection queries care about.
fn mixed_candidates() -> Vec<CleanupCandidate> {
    vec![
        CleanupCandidate {
            state: CandidateState::Present {
                size: Some(FileSize::from_bytes(1_000)),
                checked: true,
            },
            ..candidate("ticked", "/cache/ticked")
        },
        CleanupCandidate {
            state: CandidateState::Present {
                size: Some(FileSize::from_bytes(2_000)),
                checked: false,
            },
            ..candidate("unticked", "/cache/unticked")
        },
        // Absent from this machine: the dialog shows it dimmed, and it cannot
        // be ticked or hold a size.
        CleanupCandidate {
            state: CandidateState::Absent,
            ..candidate("missing", "/cache/missing")
        },
        // Ticked and present, but never measured.
        CleanupCandidate {
            state: CandidateState::Present {
                size: None,
                checked: true,
            },
            ..candidate("unmeasured", "/cache/unmeasured")
        },
    ]
}

#[test]
fn selected_paths_skip_unchecked_and_missing_candidates() {
    let dialog = dialog_with(mixed_candidates());

    assert_eq!(
        dialog.selected_paths(),
        vec![
            PathBuf::from("/cache/ticked"),
            PathBuf::from("/cache/unmeasured"),
        ],
    );
}

#[test]
fn selected_bytes_count_only_what_was_measured() {
    let dialog = dialog_with(mixed_candidates());

    assert_eq!(
        dialog.selected_bytes(),
        1_000,
        "an unmeasured candidate contributes no bytes, but is still removed"
    );
}

#[test]
fn a_fresh_dialog_offers_candidates_with_nothing_selected() {
    let dialog = DiskCleanupDialog::new();

    assert!(
        !dialog.candidates.is_empty(),
        "the checklist is built from the well-known cache locations"
    );
    assert_eq!(dialog.field, CleanupField::Item(0));
    assert!(
        dialog.candidates.iter().all(|c| c.size().is_none()),
        "sizes stay empty until `measure` runs"
    );
    assert_eq!(dialog.selected_paths(), Vec::<PathBuf>::new());
    assert_eq!(dialog.selected_bytes(), 0);
}
