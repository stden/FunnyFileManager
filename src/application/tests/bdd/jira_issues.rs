//! Purpose: Step definitions for Jira issues behavior acceptance scenarios.

mod fixture;

use std::path::Path;

use cucumber::{given, then, when};

use super::common::{open_command_row, wait_until};
use super::BehaviorWorld;
use crate::menu::MenuCommand;
use crate::models::Overlay;
use crate::test_support::{EnvVarsGuard, CONFIG_HOME};
use crate::values::jira::{JiraAccess, TOKEN_VARIABLES};

pub(super) use fixture::Fixture;

fn parts(world: &mut BehaviorWorld) -> (&mut super::Case, &mut Fixture) {
    (
        world.case.as_mut().expect("case"),
        world.jira.as_mut().expect("jira fixture"),
    )
}

#[given("configured Jira issue fixtures")]
fn configured(world: &mut BehaviorWorld) {
    world.jira = Some(Fixture::default());
}

#[given(expr = "Jira fixtures fail with {string}")]
fn failing(world: &mut BehaviorWorld, failure: String) {
    world.jira = Some(Fixture::failing(&failure));
}

#[given("a delayed Jira response")]
fn delayed(world: &mut BehaviorWorld) {
    world.jira = Some(Fixture::delayed());
}

#[given("Jira access is set to network without credentials")]
fn unconfigured_network(world: &mut BehaviorWorld) {
    world.case().app.live.jira = JiraAccess::Network;
    world.jira = Some(Fixture::default());
}

#[when("I open Jira issues from the F9 menu")]
fn open(world: &mut BehaviorWorld) {
    let case = world.case();
    let empty_cfg = case.root.path().join("empty_jira_cfg");
    let mut vars: Vec<(&'static str, &Path)> =
        vec![(CONFIG_HOME, &empty_cfg), ("JIRA_URL", Path::new(""))];
    for name in TOKEN_VARIABLES {
        vars.push((name, Path::new("")));
    }
    let _guard = (case.app.live.jira == JiraAccess::Network).then(|| EnvVarsGuard::set(&vars));
    open_command_row(case, MenuCommand::JiraIssues);
    assert!(matches!(world.case().app.overlay, Overlay::Jira(_)));
}

#[when("I start the Jira request")]
fn start(world: &mut BehaviorWorld) {
    let (case, fix) = parts(world);
    let work = fix.work();
    fix.jobs.pump_jira_with(&mut case.app, work);
    assert!(matches!(&case.app.overlay, Overlay::Jira(d) if d.is_busy()));
    case.paint();
}

#[when("the Jira request completes")]
fn complete(world: &mut BehaviorWorld) {
    let (case, fix) = parts(world);
    wait_until("jira request", || {
        let work = fix.work();
        fix.jobs.pump_jira_with(&mut case.app, work);
        matches!(&case.app.overlay, Overlay::Jira(d) if !d.is_busy() && d.queued.is_none())
    });
    case.paint();
}

#[when("the live offline Jira worker pumps")]
fn offline_pump(world: &mut BehaviorWorld) {
    let (case, fix) = parts(world);
    wait_until("offline jira", || {
        let _ = fix.jobs.pump(&mut case.app);
        matches!(&case.app.overlay, Overlay::Jira(d) if !d.is_busy() && d.queued.is_none())
    });
    case.paint();
}

#[when("I release the delayed Jira response")]
fn release(world: &mut BehaviorWorld) {
    let (case, fix) = parts(world);
    fix.gate.release();
    wait_until("delayed jira reply", || {
        let work = fix.work();
        fix.jobs.pump_jira_with(&mut case.app, work) == crate::application::Redraw::Needed
    });
    let _ = fix;
    case.paint();
}

#[then(expr = "Jira queries recorded JQL {string}")]
fn recorded_jql(world: &mut BehaviorWorld, expected: String) {
    let jqls = world
        .jira
        .as_ref()
        .expect("fixture")
        .jqls
        .lock()
        .expect("lock")
        .clone();
    assert!(jqls.iter().any(|q| q == &expected), "recorded: {jqls:?}");
}

#[then("a Jira snapshot folder exists in the active panel")]
fn snapshot_exists(world: &mut BehaviorWorld) {
    let dir = world
        .case()
        .app
        .get_active_panel_ref()
        .path
        .join("jira_exports");
    assert!(dir.is_dir(), "expected snapshot directory at {dir:?}");
}

#[then("the reopened Jira dialog still awaits its own response")]
fn reopened_waits(world: &mut BehaviorWorld) {
    let Overlay::Jira(dialog) = &world.case().app.overlay else {
        panic!("expected Jira dialog");
    };
    assert!(dialog.is_busy() || dialog.queued.is_some());
    assert_eq!(dialog.issues.len(), 0);
}
