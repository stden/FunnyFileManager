//! Unit tests: background Things patch execution, offline refusal, and worker failures.

use std::{sync::mpsc, time::Duration};

use super::{app_with, settings, state, ThingsJobs};
use crate::application::Redraw;
use crate::models::things::{ThingsAccess, ThingsState};
use crate::models::Overlay;
use crate::things::{
    build_plan, Config, GroupResult, PatchPlan, PatchReport, PatchScope, ThingsError,
};

fn empty_plan() -> PatchPlan {
    build_plan(&[], PatchScope::All, true, "patch")
}

#[test]
fn queued_patch_refuses_when_offline() {
    let mut app = app_with(ThingsState::PatchQueued {
        config: settings(),
        plan: empty_plan(),
    });
    let mut jobs = ThingsJobs::default();
    for _ in 0..5000 {
        jobs.pump(&mut app);
        if !matches!(
            state(&app),
            ThingsState::PatchQueued { .. } | ThingsState::Patching
        ) {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(matches!(state(&app), ThingsState::Failed(_)));
}

#[test]
fn queued_patch_executes_on_network_and_closes_dialog() {
    let mut app = app_with(ThingsState::PatchQueued {
        config: settings(),
        plan: empty_plan(),
    });
    app.live.things = ThingsAccess::Network;
    let mut jobs = ThingsJobs::default();
    let _ = jobs.pump_patch_with(&mut app, |_c: Config, _p: PatchPlan| {
        Ok(PatchReport {
            groups: vec![GroupResult {
                id: "g1".to_owned(),
                status: "PENDING".to_owned(),
                executions: Vec::new(),
            }],
        })
    });
    for _ in 0..5000 {
        jobs.pump_patch_with(&mut app, |_c, _p| Err(ThingsError::Disabled));
        if !app.overlay.is_active() {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!app.overlay.is_active());
}

#[test]
fn disconnected_worker_or_closed_dialog_is_safe_for_patch_jobs() {
    let mut dc_app = app_with(ThingsState::Patching);
    let generation = match &dc_app.overlay {
        Overlay::Things(d) => std::sync::Arc::clone(&d.generation),
        _ => panic!("things"),
    };
    let (tx, rx) = mpsc::channel();
    drop(tx);
    let mut jobs = ThingsJobs {
        pending: Some(super::super::Pending {
            generation,
            receiver: rx,
        }),
    };
    assert!(matches!(jobs.pump(&mut dc_app), Redraw::Needed));
    assert!(matches!(state(&dc_app), ThingsState::Failed(_)));

    let mut closed = app_with(ThingsState::Patching);
    closed.close_overlay();
    assert!(matches!(
        jobs.pump_patch_with(&mut closed, |_c, _p| Err(ThingsError::Disabled)),
        Redraw::Unchanged
    ));
}

#[test]
fn queued_network_patch_and_non_queued_patch_pump_paths() {
    let mut not_queued = app_with(ThingsState::Patching);
    let mut jobs = ThingsJobs::default();
    assert!(matches!(
        jobs.pump_patch_with(&mut not_queued, |_c, _p| Err(ThingsError::Disabled)),
        Redraw::Unchanged
    ));

    let mut app = app_with(ThingsState::PatchQueued {
        config: settings(),
        plan: empty_plan(),
    });
    app.live.things = ThingsAccess::Network;
    for _ in 0..5000 {
        jobs.pump(&mut app);
        if matches!(state(&app), ThingsState::Failed(_)) {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(matches!(state(&app), ThingsState::Failed(_)));
}
