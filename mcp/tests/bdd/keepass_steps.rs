//! Purpose: Steps for `keepass.feature`: a `KeePass` fixture saved with a
//! fast KDF, and its export through the `kbdx` tool.

use crate::world::McpWorld;
use cucumber::{given, when};
use keepass::config::KdfConfig;
use keepass::db::fields;
use keepass::{Database, DatabaseKey};
use serde_json::json;
use std::fs;

#[given(expr = "a KeePass database {string} protected by {string}")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn given_keepass_database(w: &mut McpWorld, name: String, password: String) {
    let mut database = Database::new();
    if let KdfConfig::Argon2 {
        iterations,
        memory,
        parallelism,
        ..
    } = &mut database.config.kdf_config
    {
        *iterations = 1;
        *parallelism = 1;
        *memory = 64 * 1024;
    }
    database.meta.database_name = Some("BDD Vault".to_owned());
    {
        let mut root = database.root_mut();
        let mut entry = root.add_entry();
        entry.set_unprotected(fields::TITLE, "Deploy");
        entry.set_unprotected(fields::USERNAME, "builder");
        entry.set_protected(fields::PASSWORD, "bdd-secret");
        entry.set_unprotected("Custom-Token", "custom-value");
    }

    let path = w.temp_path().join(name);
    let mut file = fs::File::create(path).expect("create KeePass fixture");
    database
        .save(&mut file, DatabaseKey::new().with_password(&password))
        .expect("save KeePass fixture");
}

#[when(expr = "I export KeePass database {string} with password {string}")]
async fn when_export_keepass_default(w: &mut McpWorld, name: String, password: String) {
    let path = w.resolve(&name);
    w.call_tool("kbdx", json!({"path": path, "password": password}));
}

#[when(expr = "I export KeePass database {string} with password {string} as {string}")]
async fn when_export_keepass_as(w: &mut McpWorld, name: String, password: String, format: String) {
    let path = w.resolve(&name);
    w.call_tool(
        "kbdx",
        json!({"path": path, "password": password, "format": format}),
    );
}
