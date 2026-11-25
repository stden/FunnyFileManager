//! Purpose: Step definitions for Cloudflare DNS acceptance scenarios.

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
        world.cloudflare.as_mut().expect("cloudflare fixture"),
    )
}

#[given("successful Cloudflare DNS zone and record fixtures")]
fn successful(world: &mut BehaviorWorld) {
    world.cloudflare = Some(Fixture::default());
}

#[given(expr = "Cloudflare DNS requests fail with {string}")]
fn failing(world: &mut BehaviorWorld, failure: String) {
    world.cloudflare = Some(Fixture::failing(&failure));
}

#[given("a delayed Cloudflare DNS response")]
fn delayed(world: &mut BehaviorWorld) {
    world.cloudflare = Some(Fixture::delayed());
}

#[when("I open Cloudflare DNS from the F9 menu")]
fn open(world: &mut BehaviorWorld) {
    open_command_row(world.case(), MenuCommand::CloudflareDns);
    assert!(matches!(
        world.case().app.overlay,
        Overlay::CloudflareDns(_)
    ));
}

#[when("I start the Cloudflare DNS request")]
fn start(world: &mut BehaviorWorld) {
    let (case, fix) = parts(world);
    let fetch = fix.fetch();
    fix.jobs.pump_with(&mut case.app, fetch);
    assert!(matches!(
        &case.app.overlay,
        Overlay::CloudflareDns(p) if p.busy.is_some()
    ));
    case.paint();
}

#[when("all Cloudflare DNS requests settle")]
fn settle(world: &mut BehaviorWorld) {
    let (case, fix) = parts(world);
    wait_until("cloudflare dns", || {
        let fetch = fix.fetch();
        fix.jobs.pump_with(&mut case.app, fetch);
        matches!(
            &case.app.overlay,
            Overlay::CloudflareDns(p) if p.busy.is_none() && p.queued.is_none()
        )
    });
    case.paint();
}

#[when("I release the delayed Cloudflare DNS response")]
fn release(world: &mut BehaviorWorld) {
    let (case, fix) = parts(world);
    fix.gate.release();
    wait_until("cloudflare delayed reply", || {
        let fetch = fix.fetch();
        fix.jobs.pump_with(&mut case.app, fetch) == crate::application::Redraw::Needed
    });
    let _ = fix;
    case.paint();
}

#[then(expr = "{int} Cloudflare DNS writes were recorded")]
fn write_count(world: &mut BehaviorWorld, expected: usize) {
    let count = world.cloudflare.as_ref().expect("fixture").write_count();
    assert_eq!(count, expected);
}

#[then("the reopened Cloudflare DNS dialog still awaits its own response")]
fn still_waiting(world: &mut BehaviorWorld) {
    let Overlay::CloudflareDns(panel) = &world.case().app.overlay else {
        panic!("expected Cloudflare DNS dialog");
    };
    assert!(panel.busy.is_some() || panel.queued.is_some());
    assert_eq!(panel.zones.len(), 0);
}
