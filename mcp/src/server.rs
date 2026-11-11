//! `FfmServer` struct definition and its constructors.
//!
//! The path-safety helpers and guard checks stay in `main.rs` because the
//! architecture contract restricts `crate::paths` access to that file.
//! The `ServerHandler` impl, the tool dispatch, and `main()` also live there.

use crate::guards;
use rmcp::handler::server::router::tool::ToolRouter;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct FfmServer {
    /// Merged tool router; accessed by the `#[tool_handler]` dispatch in `main.rs`.
    pub(super) tool_router: ToolRouter<Self>,
    /// Allowed tree for file tools and initial application-session directories.
    pub(super) root: PathBuf,
    /// Whether the server is in read-only mode (`FFM_MCP_READONLY=1`).
    pub(super) readonly: bool,
    /// Whether the kbdx tool is enabled (`FFM_MCP_ALLOW_KDBX=1`).
    pub(super) kdbx_allowed: bool,
    /// Full-application authority requires a separate explicit opt-in.
    pub(super) application_allowed: bool,
    pub(super) application: Arc<crate::automation::SessionManager>,
}

impl FfmServer {
    #[must_use]
    pub fn new(root: PathBuf) -> Self {
        let mut server = Self::with_config(root, guards::is_readonly(), guards::is_kdbx_allowed());
        server.application_allowed = guards::is_application_allowed();
        server.application = Arc::new(crate::automation::SessionManager::from_environment());
        server
    }

    /// Construct a server with explicit guard flags, bypassing the environment.
    ///
    /// Used by tests and callers that need a deterministic configuration
    /// regardless of the process environment.
    #[must_use]
    pub(crate) fn with_config(root: PathBuf, readonly: bool, kdbx_allowed: bool) -> Self {
        Self {
            tool_router: Self::tool_router(),
            root,
            readonly,
            kdbx_allowed,
            application_allowed: false,
            application: Arc::new(crate::automation::SessionManager::default()),
        }
    }
}
