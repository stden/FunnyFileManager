//! Purpose: Guard contract tests — `FFM_MCP_ALLOW_KDBX` and `FFM_MCP_READONLY`.
//!
//! Each test uses `FfmServer::with_config` so the environment is irrelevant;
//! the guard flags are explicit and deterministic.

use super::*;
use crate::tools::{
    keepass::KdbxFormat, keepass::KdbxParams, reading::ViewFileParams, writing::WriteFileParams,
};
use keepass::config::KdfConfig;
use keepass::{Database, DatabaseKey};
use rmcp::handler::server::wrapper::Parameters;
use std::fs::{self, File};
use std::path::Path;
use tempfile::TempDir;

const VAULT_PASSWORD: &str = "guards-test-master-password";

fn server_no_kdbx(dir: &TempDir) -> (FfmServer, PathBuf) {
    let root = dir.path().canonicalize().expect("canonicalize temp dir");
    (FfmServer::with_config(root.clone(), false, false), root)
}

fn server_kdbx(dir: &TempDir) -> (FfmServer, PathBuf) {
    let root = dir.path().canonicalize().expect("canonicalize temp dir");
    (FfmServer::with_config(root.clone(), false, true), root)
}

fn server_readonly(dir: &TempDir) -> (FfmServer, PathBuf) {
    let root = dir.path().canonicalize().expect("canonicalize temp dir");
    (FfmServer::with_config(root.clone(), true, false), root)
}

/// Create a minimal encrypted `KeePass` vault at `path` with a single password.
///
/// Argon2 parameters are lowered to the minimum so tests run in milliseconds.
fn create_cheap_vault(path: &Path) {
    let mut db = Database::new();
    if let KdfConfig::Argon2 {
        iterations,
        memory,
        parallelism,
        ..
    } = &mut db.config.kdf_config
    {
        *iterations = 1;
        *parallelism = 1;
        *memory = 64 * 1024;
    }
    let mut file = File::create(path).expect("create vault fixture");
    db.save(&mut file, DatabaseKey::new().with_password(VAULT_PASSWORD))
        .expect("save vault fixture");
}

fn kdbx_params(path: &Path) -> KdbxParams {
    KdbxParams {
        path: path.display().to_string(),
        password: Some(VAULT_PASSWORD.to_owned()),
        keyfile: None,
        format: KdbxFormat::Json,
    }
}

#[test]
fn kbdx_is_blocked_when_ffm_mcp_allow_kdbx_is_absent() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_no_kdbx(&dir);
    let vault = root.join("test.kdbx");
    create_cheap_vault(&vault);

    let err = server
        .kbdx(Parameters(kdbx_params(&vault)))
        .expect_err("kdbx must be blocked when kdbx_allowed=false");

    assert!(
        err.message
            .contains("kbdx is disabled. Set FFM_MCP_ALLOW_KDBX=1"),
        "unexpected error message: {}",
        err.message,
    );
}

#[test]
fn kbdx_is_allowed_when_ffm_mcp_allow_kdbx_is_set() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_kdbx(&dir);
    let vault = root.join("test.kdbx");
    create_cheap_vault(&vault);

    server
        .kbdx(Parameters(kdbx_params(&vault)))
        .expect("kdbx should succeed when kdbx_allowed=true");
}

#[test]
fn write_tool_is_blocked_in_readonly_mode() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_readonly(&dir);
    let target = root.join("blocked.txt");

    let err = server
        .write_file(Parameters(WriteFileParams {
            path: target.display().to_string(),
            content: "hello".to_owned(),
        }))
        .expect_err("write_file must be blocked in read-only mode");

    assert!(
        err.message.contains("read-only mode (FFM_MCP_READONLY=1)"),
        "unexpected error message: {}",
        err.message,
    );
}

#[test]
fn read_tool_is_allowed_in_readonly_mode() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_readonly(&dir);
    let readable = root.join("readable.txt");
    fs::write(&readable, "hello world").expect("create readable fixture");

    server
        .view_file(Parameters(ViewFileParams {
            path: readable.display().to_string(),
            offset: None,
            limit: None,
        }))
        .expect("view_file should succeed in read-only mode");
}
