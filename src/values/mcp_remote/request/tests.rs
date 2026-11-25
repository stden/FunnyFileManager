//! Purpose: Unit tests for HTTP MCP request, reply, and error formatting.

use super::*;

#[test]
fn error_display_covers_all_variants() {
    let cases = [
        (
            McpRemoteError::Disabled,
            "Unavailable in replay or isolated sessions",
        ),
        (
            McpRemoteError::MissingEnv("KEY".into()),
            "Set KEY environment variable first",
        ),
        (McpRemoteError::Config("bad cfg".into()), "bad cfg"),
        (McpRemoteError::Network("net down".into()), "net down"),
        (McpRemoteError::Protocol("bad rpc".into()), "bad rpc"),
        (
            McpRemoteError::Http(502, "bad gw".into()),
            "MCP HTTP 502: bad gw",
        ),
        (McpRemoteError::Worker, "MCP worker stopped unexpectedly"),
    ];
    for (err, expected) in cases {
        assert_eq!(err.to_string(), expected);
    }
}
