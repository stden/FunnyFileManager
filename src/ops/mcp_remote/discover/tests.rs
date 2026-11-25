//! Purpose: Discovery tests for user homes, relative folders, and live env overrides.

use std::path::Path;

use super::{discover_for, live_overrides};
use crate::mcp_clients::Host;
use crate::test_support::{EnvVarGuard, EnvVarsGuard, CONFIG_HOME};
use crate::values::mcp_remote::McpAccess;

#[test]
fn discover_reads_user_home_network_yaml_and_skips_relative_folder() {
    let temp = tempfile::TempDir::new().expect("temp");
    let claude = temp.path().join(".claude.json");
    let user_cfg = r#"{"mcpServers":{"u":{"type":"http","url":"https://mcp.example.test/u"}}}"#;
    std::fs::write(claude, user_cfg).expect("write");
    let cfg_dir = temp.path().join("cfg");
    std::fs::create_dir_all(cfg_dir.join("ffm")).expect("cfg");
    let yml = cfg_dir.join("ffm").join(crate::config::document::FILE_NAME);
    std::fs::write(
        yml,
        "mcp_servers:\n  y:\n    type: http\n    url: https://mcp.example.test/y\n",
    )
    .expect("yml");
    let _guard = EnvVarGuard::set(CONFIG_HOME, &cfg_dir);
    let host = Host {
        home: Some(temp.path().to_path_buf()),
        ..Host::default()
    };
    let found = discover_for(
        &host,
        McpAccess::Network,
        Path::new("relative"),
        None,
        None,
        None,
    );
    assert_eq!(found.len(), 2);
}

#[test]
fn live_overrides_parses_env_flags_when_network_enabled() {
    let g1 = EnvVarsGuard::set(&[
        ("FFM_MCP_CONFIGS", Path::new("extra.json")),
        ("FFM_MCP_TLS_VERIFY", Path::new("off")),
    ]);
    assert_eq!(
        live_overrides(McpAccess::Network),
        (Some("extra.json".to_owned()), Some(false))
    );
    drop(g1);

    let g2 = EnvVarGuard::set("FFM_MCP_TLS_VERIFY", Path::new("yes"));
    assert_eq!(live_overrides(McpAccess::Network).1, Some(true));
    drop(g2);

    let _g3 = EnvVarGuard::set("FFM_MCP_TLS_VERIFY", Path::new("maybe"));
    assert_eq!(live_overrides(McpAccess::Network).1, None);
}
