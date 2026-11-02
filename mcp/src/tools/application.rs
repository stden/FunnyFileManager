//! Purpose: Explicitly enabled, full-authority application sessions through private pipes.

mod actions;
mod helpers;
mod lifecycle;
pub mod params;
mod state;

use crate::FfmServer;
use rmcp::handler::server::router::tool::ToolRouter;

impl FfmServer {
    pub(super) fn application_tools() -> ToolRouter<Self> {
        Self::application_lifecycle_tools()
            + Self::application_action_tools()
            + Self::application_state_tools()
    }
}

#[cfg(test)]
mod tests;
