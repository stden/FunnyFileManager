//! The volume shown in the dialog is the one queued after confirmation.

use std::{ffi::OsString, path::PathBuf};

use crate::models::{
    App, ChoiceTarget, MaintenanceCommand, MenuDialog, Overlay, Panel, VolumeOperation,
    VolumeTarget,
};

#[test]
fn choices_default_to_cancel_and_keep_the_confirmed_volume() {
    let panel = Panel::from_entries(PathBuf::from("elsewhere"), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    let target = VolumeTarget {
        mount: PathBuf::from("C:\\"),
        id: OsString::from("test-volume"),
    };
    super::show(&mut app, &target);
    let (rows, selected) = choices(&app);
    assert!(matches!(rows[selected].target, ChoiceTarget::Close));
    assert!(app.maintenance_request.is_none());
    for (row, expected) in rows
        .iter()
        .skip(1)
        .zip([VolumeOperation::Trim, VolumeOperation::Defragment])
    {
        assert!(row.label.contains(expected.label()));
        assert!(
            matches!(&row.target, ChoiceTarget::RunMaintenance(MaintenanceCommand::Volume(op, volume)) if *op == expected && *volume == target)
        );
    }
    assert_eq!(rows.len(), 3);
    let run = rows[1].target.clone();
    crate::ops::menu::choose(&mut app, run).unwrap();
    assert_eq!(
        app.maintenance_request,
        Some(MaintenanceCommand::Volume(VolumeOperation::Trim, target))
    );
}

fn choices(app: &App) -> (&[crate::models::MenuChoice], usize) {
    let Overlay::MenuDialog(dialog) = &app.overlay else {
        panic!("expected volume dialog");
    };
    let MenuDialog::Choices { rows, selected, .. } = dialog.as_ref() else {
        panic!("expected choices");
    };
    (rows, *selected)
}

#[test]
fn git_dialog_freezes_the_repository_and_branch_before_execution() {
    use crate::ops::git_pull::tests_fixtures::{git, Fixture};
    let fixture = Fixture::new();
    let repo = fixture.clone_repo("checkout");
    git(&repo, &["switch", "-c", "feature"]);
    let panel = Panel::from_entries(repo.clone(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    crate::ops::menu::execute(&mut app, crate::menu::MenuCommand::SyncGitBranch);
    let (rows, selected) = choices(&app);
    assert!(matches!(rows[selected].target, ChoiceTarget::Close));
    assert!(rows[1].label.contains("feature"));
    let confirm = rows[1].target.clone();
    app.left_panel.path = fixture.root;
    crate::ops::menu::choose(&mut app, confirm).unwrap();
    let Some(MaintenanceCommand::SyncBranch(target)) = app.maintenance_request else {
        panic!("expected queued branch update");
    };
    assert_eq!(target.branch, "feature");
    assert_eq!(target.root, repo.canonicalize().unwrap());
}

#[test]
fn non_file_panels_are_rejected_and_main_branch_and_volume_with_open_choices() {
    use crate::ops::git_pull::tests_fixtures::Fixture;
    let fixture = Fixture::new();
    let repo = fixture.clone_repo("main-repo");
    let mut panel = Panel::from_entries(repo, Vec::new());
    panel.kind = crate::models::PanelKind::Tree(Box::default());
    let mut app = App::with_panels(panel.clone(), panel);
    assert!(super::git_sync(&mut app).is_err());

    app.left_panel.kind = crate::models::PanelKind::Files;
    super::git_sync(&mut app).expect("main branch sync dialog");
    let (rows, _) = choices(&app);
    assert_eq!(rows[1].label, "Fast-forward main to origin/main");

    let target = VolumeTarget {
        mount: PathBuf::from("C:\\"),
        id: OsString::from("vol"),
    };
    super::volume_with(&mut app, |_| Ok(target)).expect("volume dialog");
    let (rows, _) = choices(&app);
    assert_eq!(rows.len(), 3);
}
