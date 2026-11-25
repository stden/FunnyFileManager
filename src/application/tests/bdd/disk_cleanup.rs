//! Purpose: Sandboxed BDD steps for `:cleanup` candidate registration and execution.

use std::{fs, path::Path};

use cucumber::{given, then, when};

use super::BehaviorWorld;
use crate::models::{
    CandidateState, CleanupCandidate, CleanupField, CleanupKind, DiskCleanupDialog, Overlay,
};

#[then(
    "the default disk cleanup checklist includes Cargo registry and platform caches as opt-in path entries"
)]
fn default_candidates_registered(_world: &mut BehaviorWorld) {
    let dialog = DiskCleanupDialog::new();
    assert_kind(&dialog, "Temp files", CleanupKind::Contents);
    assert_kind(&dialog, "Cargo build artifacts", CleanupKind::Path);
    if crate::config::user_home().is_some() {
        assert_kind(&dialog, "Cargo registry cache", CleanupKind::Path);
        assert_kind(&dialog, "JVM heap dumps", CleanupKind::HeapDumps);
        assert_platform_caches(&dialog);
    }
}

fn assert_kind(dialog: &DiskCleanupDialog, label: &str, expected: CleanupKind) {
    let found = dialog
        .candidates
        .iter()
        .find(|candidate| candidate.label == label)
        .unwrap_or_else(|| panic!("missing cleanup candidate {label:?}"));
    assert_eq!(found.kind, expected, "unexpected kind for {label:?}");
}

fn assert_platform_caches(dialog: &DiskCleanupDialog) {
    #[cfg(windows)]
    let expected = [
        "JetBrains caches",
        "Docker WSL disk",
        "Playwright browsers",
        "pip cache",
    ];
    #[cfg(not(windows))]
    let expected = [
        "JetBrains caches",
        "Playwright browsers",
        "pip cache",
        "uv cache",
        "Hugging Face cache",
        "Puppeteer browsers",
    ];
    for label in expected {
        assert_kind(dialog, label, CleanupKind::Path);
    }
}

#[given("a sandboxed disk cleanup checklist with populated developer caches")]
fn populate_sandboxed_caches(world: &mut BehaviorWorld) {
    let base = world.case().root.path().join("cleanup_sandbox");
    fs::create_dir_all(base.join("temp")).unwrap();
    fs::write(base.join("temp/stale.tmp"), b"temp-payload").unwrap();
    fs::create_dir_all(base.join("cargo-registry/index.crates.io")).unwrap();
    fs::write(
        base.join("cargo-registry/index.crates.io/serde.crate"),
        b"crate-bytes",
    )
    .unwrap();
    fs::create_dir_all(base.join("gradle/caches")).unwrap();
    fs::write(base.join("gradle/caches/modules.bin"), b"gradle-bytes").unwrap();
    fs::create_dir_all(base.join("home")).unwrap();
    fs::write(base.join("home/java_pid1.hprof"), b"heap-dump").unwrap();
    fs::write(base.join("home/notes.txt"), b"keep-me").unwrap();
}

#[when("I open the sandboxed disk cleanup dialog")]
fn open_sandboxed_dialog(world: &mut BehaviorWorld) {
    let case = world.case();
    let base = case.root.path().join("cleanup_sandbox");
    let mut dialog = DiskCleanupDialog {
        candidates: sandbox_candidates(&base),
        field: CleanupField::Item(0),
    };
    let panel_path = case.app.get_active_panel_ref().path.clone();
    crate::ops::disk_cleanup::measure(&mut dialog, &panel_path);
    case.app.set_overlay(Overlay::DiskCleanup(Box::new(dialog)));
    case.paint();
}

fn sandbox_candidates(base: &Path) -> Vec<CleanupCandidate> {
    vec![
        row("Temp files", base.join("temp"), CleanupKind::Contents),
        row(
            "Cargo registry cache",
            base.join("cargo-registry"),
            CleanupKind::Path,
        ),
        row(
            "Gradle caches",
            base.join("gradle/caches"),
            CleanupKind::Path,
        ),
        row("JVM heap dumps", base.join("home"), CleanupKind::HeapDumps),
    ]
}

fn row(label: &'static str, path: std::path::PathBuf, kind: CleanupKind) -> CleanupCandidate {
    CleanupCandidate {
        label,
        path,
        kind,
        state: CandidateState::Absent,
    }
}

#[then(expr = "{string} and {string} are checked while {string} and {string} are unchecked")]
fn default_check_states(
    world: &mut BehaviorWorld,
    checked_a: String,
    checked_b: String,
    unchecked_a: String,
    unchecked_b: String,
) {
    let Overlay::DiskCleanup(dialog) = &world.case().app.overlay else {
        panic!("expected DiskCleanup overlay");
    };
    for label in [&checked_a, &checked_b] {
        assert!(is_checked(dialog, label), "{label:?} must start checked");
    }
    for label in [&unchecked_a, &unchecked_b] {
        assert!(!is_checked(dialog, label), "{label:?} must start unchecked");
    }
}

fn is_checked(dialog: &DiskCleanupDialog, label: &str) -> bool {
    dialog
        .candidates
        .iter()
        .find(|candidate| candidate.label == label)
        .unwrap()
        .is_checked()
}

#[then(
    expr = "the sandboxed cleanup removes {string} and {string} contents and heap dumps while keeping {string}"
)]
fn verify_selective_cleanup(
    world: &mut BehaviorWorld,
    removed_dir: String,
    emptied_dir: String,
    kept_dir: String,
) {
    assert_eq!(removed_dir, "Cargo registry cache");
    assert_eq!(emptied_dir, "Temp files");
    assert_eq!(kept_dir, "Gradle caches");
    let base = world.case().root.path().join("cleanup_sandbox");
    assert!(base.join("temp").is_dir());
    assert!(!base.join("temp/stale.tmp").exists());
    assert!(!base.join("cargo-registry").exists());
    assert!(base.join("gradle/caches/modules.bin").is_file());
    assert!(!base.join("home/java_pid1.hprof").exists());
    assert!(base.join("home/notes.txt").is_file());
}

#[then("all sandboxed developer cache files remain intact")]
fn verify_all_kept(world: &mut BehaviorWorld) {
    let base = world.case().root.path().join("cleanup_sandbox");
    assert!(base.join("temp/stale.tmp").is_file());
    assert!(base
        .join("cargo-registry/index.crates.io/serde.crate")
        .is_file());
    assert!(base.join("gradle/caches/modules.bin").is_file());
    assert!(base.join("home/java_pid1.hprof").is_file());
    assert!(base.join("home/notes.txt").is_file());
}
