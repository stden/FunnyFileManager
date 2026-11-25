//! Purpose: Step definitions for Database explorer behavior acceptance scenarios.

mod fixture;

use cucumber::{given, then, when};

use super::common::{open_command_row, wait_until};
use super::BehaviorWorld;
use crate::menu::MenuCommand;
use crate::models::Overlay;

pub(super) use fixture::Fixture;

fn ensure_fixture(world: &mut BehaviorWorld) -> &mut Fixture {
    let root = world.case().root.path().to_path_buf();
    world.database.get_or_insert_with(|| Fixture::new(&root))
}

#[given("no databases.yaml exists in the private config directory")]
fn missing_yaml(world: &mut BehaviorWorld) {
    let _ = ensure_fixture(world);
}

#[given("a malformed databases.yaml with an inline password field")]
fn malformed_yaml(world: &mut BehaviorWorld) {
    ensure_fixture(world).write_yaml("- name: bad\n  kind: postgres\n  password: secret\n");
}

#[given("a configured SQLite database with items table and active_items view")]
fn sample_sqlite(world: &mut BehaviorWorld) {
    let fix = ensure_fixture(world);
    fix.seed_sample_db();
    fix.write_sqlite_yaml("shop");
}

#[given(expr = "a configured SQLite database whose items table has {int} rows")]
fn paged_sqlite(world: &mut BehaviorWorld, count: usize) {
    let fix = ensure_fixture(world);
    fix.seed_paged_db(count);
    fix.write_sqlite_yaml("shop");
}

#[given("a configured SQLite connection pointing to a missing file")]
fn missing_sqlite_file(world: &mut BehaviorWorld) {
    ensure_fixture(world).write_sqlite_yaml("broken");
}

#[when("I open Database explorer from the F9 menu")]
fn open_from_menu(world: &mut BehaviorWorld) {
    let root = world.case().root.path().to_path_buf();
    if world.database.is_none() {
        world.database = Some(Fixture::new(&root));
    }
    let fix = world.database.as_ref().expect("fixture");
    let cfg_home = fix.cfg_home.clone();
    let case = world.case();
    let _guard = crate::test_support::EnvVarGuard::set(crate::test_support::CONFIG_HOME, &cfg_home);
    open_command_row(case, MenuCommand::DatabaseExplorer);
}

#[when("the database request completes")]
fn complete_request(world: &mut BehaviorWorld) {
    let case = world.case.as_mut().expect("case");
    let fix = world.database.as_mut().expect("database fixture");
    wait_until("database request", || {
        let _ = fix.jobs.pump(&mut case.app);
        matches!(&case.app.overlay, Overlay::Database(d) if !d.is_loading())
    });
    case.paint();
}

#[when("an orders table is added to the SQLite database on disk")]
fn add_orders(world: &mut BehaviorWorld) {
    world.database.as_ref().expect("fixture").add_orders_table();
}

#[then("the database explorer dialog is open")]
fn is_open(world: &mut BehaviorWorld) {
    assert!(
        matches!(world.case().app.overlay, Overlay::Database(_)),
        "expected Database explorer overlay"
    );
}

#[then(expr = "the database notice contains {string}")]
fn notice_contains(world: &mut BehaviorWorld, expected: String) {
    let Overlay::Database(explorer) = &world.case().app.overlay else {
        panic!("expected Database explorer overlay");
    };
    let notice = explorer.notice().unwrap_or("");
    assert!(
        notice.contains(&expected),
        "missing {expected:?} in {notice:?}"
    );
}
