//! Purpose: Unit tests for partial failure accounting and disabled-confirmation
//! F8 execution on parallel directory deletions.

use std::fs;
use std::io;
use std::path::Path;

use tempfile::TempDir;

use super::removal::{RemovalStep, FORCE_REMOVAL_FAILURE};
use crate::command::language::SettingsPage;
use crate::models::{App, Overlay, Panel};
use crate::status::Status;

struct ForcedStep;

impl ForcedStep {
    fn arm(step: RemovalStep) -> Self {
        FORCE_REMOVAL_FAILURE.set(Some(step));
        Self
    }
}

impl Drop for ForcedStep {
    fn drop(&mut self) {
        FORCE_REMOVAL_FAILURE.set(None);
    }
}

fn app_with_marks(dir: &Path, names: &[&str]) -> io::Result<App> {
    let mut panel = Panel::new(dir.to_path_buf())?;
    for &name in names {
        panel.marked.insert(name.into());
    }
    Ok(App::with_panels(panel, Panel::new(dir.to_path_buf())?))
}

#[test]
fn middle_item_failure_reports_exact_done_and_remaining_counts() -> io::Result<()> {
    let temp = TempDir::new()?;
    let ok_dir = temp.path().join("aa_empty");
    let fail_dir = temp.path().join("bb_with_file");
    let left_dir = temp.path().join("cc_untouched");
    fs::create_dir(&ok_dir)?;
    fs::create_dir(&fail_dir)?;
    fs::write(fail_dir.join("locked.txt"), b"stay")?;
    fs::create_dir(&left_dir)?;

    let mut app = app_with_marks(temp.path(), &["aa_empty", "bb_with_file", "cc_untouched"])?;
    crate::ops::handle_delete_confirm(&mut app)?;
    let _forced = ForcedStep::arm(RemovalStep::RemoveFile);
    crate::ops::handle_delete(&mut app)?;

    assert!(!ok_dir.exists() && fail_dir.join("locked.txt").exists() && left_dir.exists());
    assert!(matches!(
        app.status,
        Status::DeletedWithErrors {
            count: 1,
            error_count: 1,
            ..
        }
    ));
    let Overlay::OperationFailed(failure) = &app.overlay else {
        panic!("expected OperationFailed overlay");
    };
    assert_eq!((failure.done(), failure.left()), (1, 1));
    Ok(())
}

#[test]
fn f8_with_confirmations_disabled_deletes_tree_immediately_and_records_timings() -> io::Result<()> {
    let temp = TempDir::new()?;
    let victim = temp.path().join("instant");
    fs::create_dir_all(victim.join("nested"))?;
    fs::write(victim.join("nested").join("item.txt"), b"data")?;

    let mut app = super::tests_progress::app_on(temp.path(), "instant", temp.path());
    app.settings
        .set(SettingsPage::Confirmations, "off")
        .map_err(io::Error::other)?;
    crate::ops::handle_delete_confirm(&mut app)?;

    assert!(!victim.exists());
    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.status, Status::Deleted { count: 1 });
    let report = app.performance.report().join("\n");
    assert!(report.contains("Delete/Remove"), "{report}");
    assert!(report.contains("Delete/Refresh"), "{report}");
    Ok(())
}
