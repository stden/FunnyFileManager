//! Unit tests: F5 patch confirmation, cancel, queueing without config, and reply publishing.

use std::sync::Arc;

use tempfile::TempDir;

use super::super::{cancel_patch, confirm_patch, open_with, publish_patch, request_patch};
use super::{app, console_lines, loaded, state};
use crate::models::things::{ThingsCatalog, ThingsDialog, ThingsState};
use crate::models::Overlay;
use crate::status::Status;
use crate::things::{
    build_plan, ExecutionInfo, GroupResult, Listing, Machine, OsInfo, PatchReport, PatchScope,
    Status as MachineStatus, ThingsError,
};

fn patchable_machines() -> Vec<Machine> {
    vec![Machine {
        id: "vm-1".to_owned(),
        name: "web-01".to_owned(),
        hostname: "web-01.example.com".to_owned(),
        status: MachineStatus::Online,
        os_info: Some(OsInfo {
            name: "Red Hat Enterprise Linux 9.8".to_owned(),
            version: String::new(),
        }),
        ..Machine::default()
    }]
}

fn sample_report() -> PatchReport {
    PatchReport {
        groups: vec![GroupResult {
            id: "grp-1".to_owned(),
            status: "PENDING".to_owned(),
            executions: vec![ExecutionInfo {
                id: "ex-1".to_owned(),
                status: "PENDING".to_owned(),
            }],
        }],
    }
}

#[test]
fn request_patch_confirms_cancels_and_queues_selected_machines() {
    let dir = TempDir::new().expect("temp");
    let mut app = app(&dir, Vec::new());
    request_patch(&mut app);
    confirm_patch(&mut app);
    cancel_patch(&mut app);

    loaded(&mut app, Vec::new(), None);
    request_patch(&mut app);
    assert_eq!(
        app.status,
        Status::Error("Things: no patchable machines selected".to_owned())
    );

    open_with(&mut app, super::settings());
    if let Overlay::Things(dialog) = &mut app.overlay {
        dialog.state =
            ThingsState::Ready(ThingsCatalog::new(Listing::new(patchable_machines(), None)));
    }
    request_patch(&mut app);
    assert!(matches!(state(&app), ThingsState::ConfirmPatch { .. }));
    cancel_patch(&mut app);
    assert!(matches!(state(&app), ThingsState::Ready(_)));
    request_patch(&mut app);
    confirm_patch(&mut app);
    assert!(matches!(state(&app), ThingsState::PatchQueued { .. }));
}

#[test]
fn confirm_patch_without_stored_config_reports_failure() {
    let dir = TempDir::new().expect("temp");
    let mut app = app(&dir, Vec::new());
    let plan = build_plan(&patchable_machines(), PatchScope::All, true, "c");
    app.set_overlay(Overlay::Things(Box::new(ThingsDialog::new(
        ThingsState::ConfirmPatch {
            catalog: ThingsCatalog::new(Listing::new(Vec::new(), None)),
            plan,
        },
    ))));
    confirm_patch(&mut app);
    assert!(matches!(state(&app), ThingsState::Failed(_)));
    drop(ThingsDialog::new(ThingsState::Patching));
}

#[test]
fn publish_patch_logs_groups_closes_dialog_and_ignores_stale_replies() {
    let dir = TempDir::new().expect("temp");
    let mut app = app(&dir, Vec::new());
    let generation = Arc::new(());
    publish_patch(&mut app, &generation, Ok(sample_report()));
    assert!(!app.overlay.is_active());

    loaded(&mut app, patchable_machines(), None);
    publish_patch(&mut app, &generation, Err(ThingsError::Request));
    assert!(matches!(state(&app), ThingsState::Ready(_)));

    let live_gen = match &app.overlay {
        Overlay::Things(d) => Arc::clone(&d.generation),
        _ => panic!("things"),
    };
    publish_patch(&mut app, &live_gen, Err(ThingsError::Request));
    assert!(matches!(state(&app), ThingsState::Failed(_)));

    publish_patch(&mut app, &live_gen, Ok(sample_report()));
    assert!(!app.overlay.is_active());
    assert!(console_lines(&app).iter().any(|l| l.contains("grp-1")));
}

#[test]
fn patch_transitions_preserve_non_matching_dialog_states() {
    let dir = TempDir::new().expect("temp");
    let mut app = app(&dir, Vec::new());

    loaded(&mut app, patchable_machines(), None);
    cancel_patch(&mut app);
    assert!(matches!(state(&app), ThingsState::Ready(_)));
    confirm_patch(&mut app);
    assert!(matches!(state(&app), ThingsState::Ready(_)));

    if let Overlay::Things(dialog) = &mut app.overlay {
        dialog.state = ThingsState::Failed("earlier failure".to_owned());
    }
    request_patch(&mut app);
    assert!(matches!(state(&app), ThingsState::Failed(_)));
}
