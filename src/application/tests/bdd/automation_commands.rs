//! Purpose: BDD steps for typed automation command resolution and modal context enforcement.

use std::io;

use cucumber::{then, when};

use super::BehaviorWorld;
use crate::{
    application::automation::{current_context, invoke},
    automation::catalog::{self, LookupError},
    models::TaskTab,
};

#[when(expr = "I invoke automation command {string} {string}")]
fn invoke_command(world: &mut BehaviorWorld, context: String, action: String) {
    let invocation = catalog::resolve(&context, &action).expect("known catalog command");
    let case = world.case();
    invoke(&mut case.terminal, &mut case.app, invocation).expect("invocation succeeds");
    case.paint();
}

#[then(expr = "the active automation context is {string}")]
fn active_context_is(world: &mut BehaviorWorld, expected: String) {
    let actual = current_context(&world.case().app).map(crate::keymap::Context::name);
    assert_eq!(actual, Some(expected.as_str()));
}

#[then(expr = "automation command {string} {string} is rejected for the current context")]
fn rejected_for_context(world: &mut BehaviorWorld, context: String, action: String) {
    let invocation = catalog::resolve(&context, &action).expect("known catalog command");
    let case = world.case();
    let err = invoke(&mut case.terminal, &mut case.app, invocation)
        .expect_err("out-of-context invocation must fail");
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
}

#[then(expr = "automation lookup {string} {string} fails with UnknownContext")]
fn lookup_unknown_context(_world: &mut BehaviorWorld, context: String, action: String) {
    assert_eq!(
        catalog::resolve(&context, &action).err(),
        Some(LookupError::UnknownContext)
    );
}

#[then(expr = "automation lookup {string} {string} fails with UnknownAction")]
fn lookup_unknown_action(_world: &mut BehaviorWorld, context: String, action: String) {
    assert_eq!(
        catalog::resolve(&context, &action).err(),
        Some(LookupError::UnknownAction)
    );
}

#[then(expr = "the task list displays the {string} tab")]
fn task_list_tab_equals(world: &mut BehaviorWorld, expected: String) {
    let tab = world
        .case()
        .app
        .task_list_mut()
        .expect("task list open")
        .tab();
    let expected_tab = match expected.as_str() {
        "Processes" => TaskTab::Processes,
        "Network" => TaskTab::Network,
        _ => panic!("unknown task tab {expected:?}"),
    };
    assert_eq!(tab, expected_tab);
}
