//! Purpose: Temporary-cleanup acceptance steps use real menu input and a TempDir-only worker.
use super::{BehaviorWorld, Case};
use crate::application::system_temp_cleanup_jobs::TempCleanupJobs;
use crate::models::{
    temp_cleanup::{TempCleanupState, TempSource},
    Overlay,
};
use cucumber::{given, when};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

mod checks;
mod fixture;
pub(super) use fixture::Fixture;

#[given("an isolated temporary-file cleanup workspace")]
fn temporary_files(world: &mut BehaviorWorld) {
    let fixture = Fixture::new(world.case());
    world.case().app.live.temp_cleanup = TempSource::Directory {
        path: fixture.directory.clone(),
        home: Some(fixture.home.clone()),
    };
    world.temp_cleanup = Some(fixture);
}

#[when("I open temporary-file cleanup from the F9 menu")]
fn open(world: &mut BehaviorWorld) {
    let case = world.case();
    case.press("F9 Alt+C Home");
    select_cleanup(case);
    case.press("Enter");
    assert!(matches!(case.app.overlay, Overlay::TempCleanup(_)));
}

fn select_cleanup(case: &mut Case) {
    use crate::{action::Action, menu::MenuCommand};
    for _ in 0..100 {
        let Overlay::Menu(menu) = &case.app.overlay else {
            panic!("the F9 Commands menu must be open");
        };
        if menu.entry().and_then(crate::menu::Entry::action)
            == Some(Action::Menu(MenuCommand::SystemTempCleanup))
        {
            return;
        }
        case.press("Down");
    }
    panic!("the menu must offer temporary-file cleanup");
}

#[derive(Clone, Copy)]
enum Awaited {
    Preview,
    Finished,
}

#[when("the background temporary-file preview completes")]
fn preview(world: &mut BehaviorWorld) {
    complete(world, Awaited::Preview);
}

#[when("the background temporary-file cleanup completes")]
fn deletion(world: &mut BehaviorWorld) {
    complete(world, Awaited::Finished);
}

fn complete(world: &mut BehaviorWorld, awaited: Awaited) {
    let (case, fixture) = parts(world);
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        fixture.jobs.pump(&mut case.app);
        if ready(case, awaited) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "temporary cleanup worker did not finish"
        );
        std::thread::yield_now();
    }
    case.paint();
}

fn ready(case: &Case, awaited: Awaited) -> bool {
    let Overlay::TempCleanup(dialog) = &case.app.overlay else {
        panic!("the temporary cleanup dialog must stay open");
    };
    match (&dialog.state, awaited) {
        (TempCleanupState::Preview { .. }, Awaited::Preview)
        | (TempCleanupState::Finished(_), Awaited::Finished) => true,
        (TempCleanupState::Failed(error), _) => panic!("fixture worker failed: {error:?}"),
        _ => false,
    }
}

#[when("the old temporary file changes after preview")]
fn changed(world: &mut BehaviorWorld) {
    world.temp_cleanup.as_mut().unwrap().change_old_file();
}

fn parts(world: &mut BehaviorWorld) -> (&mut Case, &mut Fixture) {
    (
        world.case.as_mut().unwrap(),
        world.temp_cleanup.as_mut().unwrap(),
    )
}

fn old_file(world: &mut BehaviorWorld) -> PathBuf {
    world.temp_cleanup.as_ref().unwrap().old.clone()
}
