//! Purpose: Things catalog acceptance steps: the F9 Commands row, the read answered
//! through the job seam, the rows and marks the dialog shows, and the sessions an
//! import saves.

use std::path::PathBuf;

use cucumber::{given, then, when};

use super::common::open_command_row;
use super::{BehaviorWorld, Case};
use crate::{
    application::{things_jobs::ThingsJobs, Redraw},
    menu::MenuCommand,
    models::{things::ThingsState, Overlay},
    ops::sessions,
    sessions::SessionStore,
    things::ThingsError,
};

mod reads;
mod settings;

use reads::{machines, settle};
use settings::with_catalog_settings;

/// The job seam a scenario reads through, and whether the catalog's variables are set.
pub(super) struct Fixture {
    jobs: ThingsJobs,
    configured: bool,
}

fn fixture(world: &mut BehaviorWorld) -> &mut Fixture {
    world
        .things
        .as_mut()
        .expect("the private ffm.yml Given runs first")
}

fn parts(world: &mut BehaviorWorld) -> (&mut Case, &mut Fixture) {
    (world.case.as_mut().unwrap(), world.things.as_mut().unwrap())
}

fn saved_path(case: &Case) -> PathBuf {
    case.root.path().join("ffm.yml")
}

/// The saved sessions, which must load: a damaged file reads as an empty store,
/// so an empty check alone would pass on it.
fn loaded_store(case: &Case) -> SessionStore {
    let store = SessionStore::load_from(saved_path(case));
    assert!(
        store.damage().is_none(),
        "the saved sessions do not load again: {:?}",
        store.damage()
    );
    store
}

#[given("a private ffm.yml for the Things sessions")]
fn private_sessions(world: &mut BehaviorWorld) {
    let case = world.case();
    let path = saved_path(case);
    sessions::install(&mut case.app, SessionStore::load_from(path));
    world.things = Some(Fixture {
        jobs: ThingsJobs::default(),
        configured: false,
    });
}

#[given("the Things settings are unset")]
fn unset_settings(world: &mut BehaviorWorld) {
    fixture(world).configured = false;
}

#[given("the Things settings are configured")]
fn configured_settings(world: &mut BehaviorWorld) {
    fixture(world).configured = true;
}

#[when("I choose the Things catalog from the F9 menu")]
fn choose(world: &mut BehaviorWorld) {
    let configured = fixture(world).configured;
    let case = world.case();
    with_catalog_settings(configured, || {
        open_command_row(case, MenuCommand::ThingsCatalog);
    });
}

#[then("the Things catalog dialog is open")]
fn dialog_is_open(world: &mut BehaviorWorld) {
    assert!(
        matches!(world.case().app.overlay, Overlay::Things(_)),
        "the Things catalog dialog must be open"
    );
}

#[when(expr = "the Things read returns the machines {string}")]
fn read_returns(world: &mut BehaviorWorld, names: String) {
    let (case, fixture) = parts(world);
    settle(&mut case.app, &mut fixture.jobs, Ok(machines(&names)));
    case.paint();
    assert!(
        matches!(&case.app.overlay, Overlay::Things(dialog) if matches!(dialog.state, ThingsState::Ready(_))),
        "the Things read did not produce a catalog\n{}",
        case.screen.text()
    );
}

#[given(expr = "the Things catalog lists the machines {string}")]
fn catalog_lists(world: &mut BehaviorWorld, names: String) {
    choose(world);
    read_returns(world, names);
}

#[then("no Things read starts")]
fn no_read_starts(world: &mut BehaviorWorld) {
    let (case, fixture) = parts(world);
    let redraw = fixture
        .jobs
        .pump_with(&mut case.app, |_config| Err(ThingsError::Disabled));
    assert_eq!(
        redraw,
        Redraw::Unchanged,
        "a read started with no request queued"
    );
}

#[then(expr = "the Things list shows a row for {string}")]
fn list_shows(world: &mut BehaviorWorld, name: String) {
    let screen = world.case().screen.text();
    let host = format!("{name}-host.example.test");
    assert!(
        screen
            .lines()
            .any(|line| line.contains(&name) && line.contains(&host)),
        "no row for {name:?}\n{screen}"
    );
}

#[then(expr = "the Things list has no row for {string}")]
fn list_hides(world: &mut BehaviorWorld, name: String) {
    let screen = world.case().screen.text();
    assert!(
        !screen.lines().any(|line| line.contains(&name)),
        "a row for {name:?} is listed\n{screen}"
    );
}

#[then(expr = "the Things list marks the row for {string}")]
fn list_marks(world: &mut BehaviorWorld, name: String) {
    let screen = world.case().screen.text();
    let marked = format!("* {name}");
    assert!(
        screen.lines().any(|line| line.contains(&marked)),
        "{name:?} is not marked\n{screen}"
    );
}

#[then(expr = "the saved Things sessions are {string}")]
fn saved_sessions(world: &mut BehaviorWorld, expected: String) {
    let store = loaded_store(world.case());
    let names: Vec<&str> = store
        .sorted()
        .into_iter()
        .map(|session| session.name.as_str())
        .collect();
    assert_eq!(names.join(" "), expected);
}

#[then("no Things sessions are saved")]
fn no_sessions_saved(world: &mut BehaviorWorld) {
    let store = loaded_store(world.case());
    assert!(store.is_empty(), "the saved store holds sessions");
}
