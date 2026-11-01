//! Purpose: Token reports connect menu input, fixture workers and rendered provider rows.

use super::{BehaviorWorld, Case};
use crate::models::{token_usage::UsageState, Overlay};
use cucumber::{given, then, when};
use std::time::{Duration, Instant};

mod checks;
mod fixture;
mod replies;
pub(super) use fixture::Fixture;

#[given("successful provider usage fixtures")]
fn successful(world: &mut BehaviorWorld) {
    world.usage = Some(Fixture::default());
}

#[given(expr = "provider usage fixtures fail with {string}")]
fn failing(world: &mut BehaviorWorld, failure: String) {
    world.usage = Some(Fixture::failing(&failure));
}

#[given("a delayed provider usage response")]
fn delayed(world: &mut BehaviorWorld) {
    world.usage = Some(Fixture::delayed());
}

#[when("I open token usage from the menu")]
fn open(world: &mut BehaviorWorld) {
    let case = world.case();
    case.press("F9 Alt+C Home");
    select_usage_row(case);
    case.press("Enter");
    assert!(matches!(case.app.overlay, Overlay::TokenUsage(_)));
}

fn select_usage_row(case: &mut Case) {
    use crate::{action::Action, menu::MenuCommand};
    for _ in 0..100 {
        let Overlay::Menu(menu) = &case.app.overlay else {
            panic!("the Commands menu must be open");
        };
        if menu.entry().and_then(crate::menu::Entry::action)
            == Some(Action::Menu(MenuCommand::TokenUsage))
        {
            return;
        }
        case.press("Down");
    }
    panic!("the Commands menu must offer Token usage");
}

#[when("I start the fixture usage refresh")]
fn start(world: &mut BehaviorWorld) {
    let (case, fixture) = parts(world);
    let fetch = fixture.fetch();
    fixture.jobs.pump_with(&mut case.app, fetch);
    assert_eq!(state(case), UsageState::Loading);
    case.paint();
}

#[when("the fixture usage response completes")]
fn complete(world: &mut BehaviorWorld) {
    let (case, fixture) = parts(world);
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let fetch = fixture.fetch();
        fixture.jobs.pump_with(&mut case.app, fetch);
        if state(case) == UsageState::Ready {
            break;
        }
        assert!(Instant::now() < deadline, "usage fixture did not finish");
        std::thread::yield_now();
    }
    case.paint();
}

#[when("new provider usage is available")]
fn newer(world: &mut BehaviorWorld) {
    world.usage.as_mut().unwrap().refresh();
}

#[when("I release the prior usage response")]
fn release(world: &mut BehaviorWorld) {
    let (case, fixture) = parts(world);
    fixture.release();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let fetch = fixture.fetch();
        if fixture.jobs.pump_with(&mut case.app, fetch) == crate::application::Redraw::Needed {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "prior usage reply did not finish"
        );
        std::thread::yield_now();
    }
    case.paint();
}

#[then("the token usage panel remains closed")]
fn closed(world: &mut BehaviorWorld) {
    assert!(matches!(world.case().app.overlay, Overlay::None));
}

#[then("the new token usage panel is still waiting for its own report")]
fn new_panel_waits(world: &mut BehaviorWorld) {
    let case = world.case();
    assert_eq!(state(case), UsageState::Loading);
    assert!(case.screen.text().contains("Loading provider reports"));
}

fn parts(world: &mut BehaviorWorld) -> (&mut Case, &mut Fixture) {
    (
        world.case.as_mut().expect("workspace Given must run first"),
        world
            .usage
            .as_mut()
            .expect("usage fixture Given must run first"),
    )
}

fn state(case: &Case) -> UsageState {
    let Overlay::TokenUsage(panel) = &case.app.overlay else {
        panic!("token usage panel must remain open");
    };
    panel.state
}
