//! Purpose: Step definitions for AI assistant behavior acceptance scenarios.

mod fixture;

use cucumber::{given, then, when};

use super::common::{open_command_row, wait_until};
use super::BehaviorWorld;
use crate::menu::MenuCommand;
use crate::models::Overlay;

pub(super) use fixture::Fixture;

fn parts(world: &mut BehaviorWorld) -> (&mut super::Case, &mut Fixture) {
    (
        world.case.as_mut().expect("case"),
        world.ai.as_mut().expect("ai fixture"),
    )
}

#[given("configured AI assistant fixtures")]
fn configured(world: &mut BehaviorWorld) {
    world.ai = Some(Fixture::default());
}

#[given(expr = "AI assistant fixtures fail with {string}")]
fn failing(world: &mut BehaviorWorld, failure: String) {
    world.ai = Some(Fixture::failing(&failure));
}

#[given("a delayed AI assistant response")]
fn delayed(world: &mut BehaviorWorld) {
    world.ai = Some(Fixture::delayed());
}

#[when("I open AI assistant from the F9 menu")]
fn open(world: &mut BehaviorWorld) {
    open_command_row(world.case(), MenuCommand::AiAssistant);
    assert!(matches!(world.case().app.overlay, Overlay::Ai(_)));
}

#[when(expr = "I select AI provider {string}")]
fn select_provider(world: &mut BehaviorWorld, key: String) {
    world.case().press(&key);
}

#[when("I start the AI worker")]
fn start_worker(world: &mut BehaviorWorld) {
    let (case, fix) = parts(world);
    let work = fix.work();
    fix.jobs.pump_ai_with(&mut case.app, work);
    assert!(matches!(&case.app.overlay, Overlay::Ai(d) if d.busy));
    case.paint();
}

#[when("the AI request completes")]
fn complete(world: &mut BehaviorWorld) {
    let (case, fix) = parts(world);
    wait_until("ai request", || {
        let work = fix.work();
        fix.jobs.pump_ai_with(&mut case.app, work);
        matches!(&case.app.overlay, Overlay::Ai(d) if !d.busy && d.queued.is_none())
    });
    case.paint();
}

#[when("the live offline AI worker pumps")]
fn offline_pump(world: &mut BehaviorWorld) {
    let (case, fix) = parts(world);
    wait_until("offline ai", || {
        let _ = fix.jobs.pump(&mut case.app);
        matches!(&case.app.overlay, Overlay::Ai(d) if !d.busy && d.queued.is_none())
    });
    case.paint();
}

#[when("I release the delayed AI response")]
fn release(world: &mut BehaviorWorld) {
    let (case, fix) = parts(world);
    fix.gate.release();
    wait_until("delayed ai reply", || {
        let work = fix.work();
        fix.jobs.pump_ai_with(&mut case.app, work) == crate::application::Redraw::Needed
    });
    let _ = fix;
    case.paint();
}

#[then(expr = "the AI transport recorded endpoint {string}")]
fn recorded_endpoint(world: &mut BehaviorWorld, expected: String) {
    let eps = world
        .ai
        .as_ref()
        .expect("fixture")
        .endpoints
        .lock()
        .expect("lock")
        .clone();
    assert!(eps.iter().any(|p| p == &expected), "recorded: {eps:?}");
}

#[then(expr = "the command line contains {string}")]
fn command_line_contains(world: &mut BehaviorWorld, expected: String) {
    let Overlay::CommandLine(cmd) = &world.case().app.overlay else {
        panic!("expected active command line overlay");
    };
    assert_eq!(cmd.line.text(), expected);
}

#[then(expr = "the active panel contains {string} with {string}")]
fn active_panel_file(world: &mut BehaviorWorld, name: String, snippet: String) {
    let path = world.case().app.get_active_panel_ref().path.join(&name);
    let body = std::fs::read_to_string(&path).expect("saved file must exist");
    assert!(body.contains(&snippet), "missing {snippet:?} in {body}");
}

#[then("the reopened AI dialog remains idle and empty")]
fn reopened_idle_empty(world: &mut BehaviorWorld) {
    let Overlay::Ai(dialog) = &world.case().app.overlay else {
        panic!("expected reopened AI dialog");
    };
    assert!(!dialog.busy);
    assert_eq!(dialog.messages.len(), 0);
}
