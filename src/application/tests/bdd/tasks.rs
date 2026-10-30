//! Task controls run against the recording host, on every test platform.

use super::{samples, BehaviorWorld, Case};
use crate::{
    models::{
        ControlError, Modal, Overlay, Pid, Priority, PriorityChange, Sample, Signal, TaskList,
    },
    test_support::processes::{one_core, Control},
};
use cucumber::{given, then};
use std::time::Instant;

pub(super) fn apply(case: &mut Case, sample: Sample) {
    let list = case.app.task_list_mut().unwrap();
    list.refresh_now();
    list.take_request(Instant::now());
    list.apply_sample(Ok(sample), Instant::now());
    case.paint();
}

#[given(expr = "the task list contains a Windows process with {word} priority")]
fn windows_process(world: &mut BehaviorWorld, class: String) {
    let case = world.case();
    case.press("Ctrl+W");
    let list = TaskList::new(
        case.app.task_host.next_session(),
        Pid::new(900).unwrap(),
        crate::prompt::Privilege::User,
    );
    case.app.set_overlay(Overlay::Tasks(Box::new(list)));
    apply(
        case,
        one_core(vec![samples::worker(samples::class(&class))]),
    );
}

#[given("the process host refuses priority changes")]
fn denied(world: &mut BehaviorWorld) {
    let key = samples::worker(samples::class("Normal")).key;
    world.case().host.fail(key, ControlError::PermissionDenied);
}

#[then(expr = "the fixture process priority is {word}")]
fn priority(world: &mut BehaviorWorld, name: String) {
    let list = world.case().app.task_list_mut().unwrap();
    let key = list.cursor().unwrap().key();
    assert_eq!(
        list.row(key).unwrap().sample().priority,
        Priority::Windows {
            class: samples::class(&name)
        }
    );
}

#[then(expr = "only class {word} was requested from the process host")]
fn requested(world: &mut BehaviorWorld, name: String) {
    let key = samples::worker(samples::class("Normal")).key;
    let change = PriorityChange::Class(samples::class(&name).settable().unwrap());
    assert_eq!(
        world.case().host.controls(),
        vec![Control::Priority(key, change)]
    );
}

#[then("no process control was requested")]
fn no_control(world: &mut BehaviorWorld) {
    assert_eq!(world.case().host.controls(), []);
}

#[then("termination is awaiting confirmation")]
fn confirmation(world: &mut BehaviorWorld) {
    assert!(matches!(
        world.case().app.task_list_mut().unwrap().modal(),
        Modal::Confirm(_)
    ));
}

#[then("only the selected fixture process received termination")]
fn terminated(world: &mut BehaviorWorld) {
    let key = samples::worker(samples::class("Normal")).key;
    assert_eq!(
        world.case().host.controls(),
        vec![Control::Signal(key, Signal::Terminate)]
    );
}
