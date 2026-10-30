//! Purpose: Server-wide security guards: feature flags read once from the
//! environment at startup and stored on `FfmServer`.
//!
//! All guard logic is pure (given a bool, decide) so it is testable without
//! touching the environment. The env-reading helpers are called only in
//! `FfmServer::new` and in integration tests that construct a server whose
//! config must be environment-driven.

use rmcp::ErrorData as McpError;

/// Returns `true` when `FFM_MCP_READONLY=1`.
///
/// Called once at startup — store the result on [`crate::FfmServer`] rather
/// than polling the environment on every request.
pub fn is_readonly() -> bool {
    std::env::var("FFM_MCP_READONLY").as_deref() == Ok("1")
}

/// Returns `true` when `FFM_MCP_ALLOW_KDBX=1`.
///
/// Called once at startup — store the result on [`crate::FfmServer`] rather
/// than polling the environment on every request.
pub fn is_kdbx_allowed() -> bool {
    std::env::var("FFM_MCP_ALLOW_KDBX").as_deref() == Ok("1")
}

/// Returns an error when `readonly` is `true`.
///
/// The `readonly` argument is the value stored on the server, not a fresh env
/// read — callers must not pass `is_readonly()` here directly.
pub fn require_readwrite(readonly: bool) -> Result<(), McpError> {
    if readonly {
        return Err(McpError::invalid_params(
            "Server is in read-only mode (FFM_MCP_READONLY=1).",
            None,
        ));
    }
    Ok(())
}

/// Returns an error when `allowed` is `false`.
///
/// The `allowed` argument is the value stored on the server, not a fresh env
/// read — callers must not pass `is_kdbx_allowed()` here directly.
pub fn require_kdbx_allowed(allowed: bool) -> Result<(), McpError> {
    if !allowed {
        return Err(McpError::invalid_params(
            "kbdx is disabled. Set FFM_MCP_ALLOW_KDBX=1 to enable plaintext vault export.",
            None,
        ));
    }
    Ok(())
}
