//! Purpose: Temporary cleanup waits for workers and returns at preview and confirmation.

use super::*;
use crate::models::{
    temp_cleanup::{
        FileIdentity, TempChoice, TempCleanupDialog, TempCutoff, TempError, TempOutcome, TempPlan,
        TempRoot, TempSource,
    },
    Confirm, FileSize,
};
use std::time::SystemTime;

fn plan(root: &TempDir) -> TempPlan {
    TempPlan {
        root: TempRoot {
            path: root.path().to_path_buf(),
            identity: FileIdentity(file_id::get_file_id(root.path()).unwrap()),
        },
        cutoff: TempCutoff(SystemTime::UNIX_EPOCH),
        files: Vec::new(),
        bytes: FileSize::from_bytes(0),
        skipped: 0,
    }
}

fn cases(plan: TempPlan) -> [(TempCleanupState, bool); 8] {
    let preview = TempCleanupState::Preview {
        plan: plan.clone(),
        focus: TempChoice::Cancel,
    };
    let confirm = TempCleanupState::Confirm {
        plan: plan.clone(),
        answer: Confirm::No,
    };
    [
        (TempCleanupState::QueuedScan(TempSource::Unavailable), true),
        (TempCleanupState::Scanning, true),
        (TempCleanupState::QueuedDelete(plan), true),
        (TempCleanupState::Deleting, true),
        (preview, false),
        (confirm, false),
        (TempCleanupState::Finished(TempOutcome::default()), false),
        (TempCleanupState::Failed(TempError::Unavailable), false),
    ]
}

#[test]
fn cleanup_is_pending_only_while_scanning_or_deleting() {
    let root = TempDir::new().unwrap();
    let mut app = application(&root);
    for (state, pending) in cases(plan(&root)) {
        let mut dialog = TempCleanupDialog::new(TempSource::Unavailable);
        dialog.state = state;
        app.overlay = Overlay::TempCleanup(Box::new(dialog));
        assert_eq!(is_pending(&app), pending);
    }
}
