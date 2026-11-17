use std::path::Path;

use super::TuiWorld;
use cucumber::given;
use keepass::{config::KdfConfig, db::fields, Database, DatabaseKey};

#[given("a password protected vault with nested and empty groups")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "Cucumber dictates step signatures: the world arrives as `&mut`"
)]
fn vault(world: &mut TuiWorld) {
    write_vault(&world.path("left", "passwords.kbdx"));
}

#[given("a password protected vault nested under a long directory path")]
#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "Cucumber dictates step signatures: the world arrives as `&mut`"
)]
fn nested_vault(world: &mut TuiWorld) {
    let nested = world.path(
        "left",
        "archive-workspace-alpha/credentials-production-europe/cluster-vaults/passwords.kbdx",
    );
    write_vault(&nested);
}

#[expect(
    clippy::expect_used,
    reason = "a Cucumber step fails by panicking, as a #[test] does"
)]
fn write_vault(path: &Path) {
    let mut db = Database::new();
    if let KdfConfig::Argon2 {
        iterations,
        memory,
        parallelism,
        ..
    } = &mut db.config.kdf_config
    {
        *iterations = 1;
        *memory = 64 * 1024;
        *parallelism = 1;
    }
    populate(&mut db);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("vault parent directory");
    }
    let mut file = std::fs::File::create(path).expect("vault file");
    db.save(&mut file, DatabaseKey::new().with_password("test-master"))
        .expect("encrypted fixture");
}

fn populate(db: &mut Database) {
    let mut root = db.root_mut();
    "Empty group".clone_into(&mut root.add_group().name);
    let mut work = root.add_group();
    "Work".clone_into(&mut work.name);
    let mut nested = work.add_group();
    "Servers".clone_into(&mut nested.name);
    let mut entry = nested.add_entry();
    entry.set_unprotected(fields::TITLE, "Build server");
    entry.set_unprotected(fields::USERNAME, "builder");
    entry.set_protected(fields::PASSWORD, "fixture-password");
}
