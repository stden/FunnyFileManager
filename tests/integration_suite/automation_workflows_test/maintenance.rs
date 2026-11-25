//! Maintenance actions show their actual target before doing any work.

use super::{screen, Session};
use std::process::Command;

#[test]
fn git_sync_names_the_current_branch_and_cancels_without_an_upstream() {
    let root = tempfile::tempdir().unwrap();
    init_topic(root.path());
    let mut session = Session::start(root.path());
    let dialog = screen(&session.action("panels", "sync_git_branch"));
    assert!(
        dialog.contains("Pull topic, update main and rebase onto origin/main"),
        "{dialog}"
    );
    assert!(dialog.contains("Cancel"));
    assert_eq!(session.keys(&["Enter"])["modal"]["kind"], "none");
    session.close();
}

fn init_topic(root: &std::path::Path) {
    for args in [
        &["init", "--initial-branch=topic"][..],
        &["commit", "--allow-empty", "-m", "initial"],
    ] {
        let result = Command::new("git")
            .args([
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=test@example.invalid",
            ])
            .args(["-c", "commit.gpgsign=false"])
            .args(args)
            .current_dir(root)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

#[cfg(windows)]
#[test]
fn volume_menu_offers_both_operations_with_an_ordinary_drive_path() {
    let root = tempfile::tempdir().unwrap();
    let mut session = Session::start(root.path());
    let dialog = screen(&session.action("panels", "optimize_volume"));
    for operation in ["TRIM ", "Defragment "] {
        let row = dialog
            .lines()
            .find(|line| line.contains(operation))
            .unwrap();
        assert!(!row.contains(r"\\?\"), "{row}");
    }
    assert_eq!(session.keys(&["Esc"])["modal"]["kind"], "none");
    session.close();
}
