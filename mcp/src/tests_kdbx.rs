//! In-process contract tests for the secret-bearing `kbdx` MCP tool.

use super::*;
use crate::tools::keepass::{KdbxFormat, KdbxParams};
use keepass::config::KdfConfig;
use keepass::db::fields;
use keepass::{Database, DatabaseKey};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use std::fs::File;
use std::path::Path;
use tempfile::TempDir;

const MASTER_PASSWORD: &str = "synthetic-master-password";

fn server_at(dir: &TempDir) -> (FfmServer, PathBuf) {
    let root = dir.path().canonicalize().expect("canonicalize temp dir");
    (FfmServer::new(root.clone()), root)
}

fn cheap_database() -> Database {
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
    database
}

fn fixture_database() -> Database {
    let mut database = cheap_database();
    database.meta.database_name = Some("Synthetic Vault".to_owned());
    database.meta.database_description = Some("MCP test data".to_owned());
    {
        let mut root = database.root_mut();
        root.name = "Root".to_owned();

        let mut github = root.add_entry();
        github.set_unprotected(fields::TITLE, "GitHub");
        github.set_unprotected(fields::USERNAME, "alice");
        github.set_protected(fields::PASSWORD, "root-secret");
        github.set_unprotected("API-Token", "custom-secret");

        let mut work = root.add_group();
        work.name = "Work".to_owned();

        let mut first = work.add_entry();
        first.set_unprotected(fields::TITLE, "API token");
        first.set_protected(fields::PASSWORD, "a\\b\"c\n${TOKEN}\t");

        let mut second = work.add_entry();
        second.set_unprotected(fields::TITLE, "API-token");
        second.set_protected(fields::PASSWORD, "second-secret");
    }
    database
}

fn save_database(path: &Path, database: &Database, key: DatabaseKey) {
    let mut file = File::create(path).expect("create encrypted fixture");
    database
        .save(&mut file, key)
        .expect("save encrypted fixture");
}

fn save_password_fixture(path: &Path) {
    save_database(
        path,
        &fixture_database(),
        DatabaseKey::new().with_password(MASTER_PASSWORD),
    );
}

fn tool_text(result: &CallToolResult) -> &str {
    &result
        .content
        .first()
        .and_then(|content| content.as_text())
        .expect("a successful tool result has text content")
        .text
}

fn params(path: &Path, format: KdbxFormat) -> KdbxParams {
    KdbxParams {
        path: path.display().to_string(),
        password: Some(MASTER_PASSWORD.to_owned()),
        keyfile: None,
        format,
    }
}

fn entry_named<'a>(document: &'a serde_json::Value, title: &str) -> &'a serde_json::Value {
    document["entries"]
        .as_object()
        .expect("serialized database has an entry map")
        .values()
        .find(|entry| entry["fields"][fields::TITLE] == title)
        .expect("entry title is present")
}

mod tests_formats;
mod tests_refusals;
