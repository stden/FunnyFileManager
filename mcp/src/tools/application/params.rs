//! Purpose: MCP schemas for the explicitly enabled full-application sessions.

use crate::automation::{MouseInput, Profile, SessionId};
use serde::Deserialize;

#[derive(Deserialize, schemars::JsonSchema)]
pub struct StartParams {
    /// Initial directory inside the file-tool root. Later application navigation is unrestricted.
    pub(crate) path: Option<String>,
    /// Terminal width, 20..400; default 120.
    #[serde(default = "default_width")]
    pub(crate) width: u16,
    /// Terminal height, 8..200; default 40.
    #[serde(default = "default_height")]
    pub(crate) height: u16,
    /// User configuration/integrations (`system`) or a separate workspace (`isolated`).
    #[serde(default)]
    pub(crate) profile: Profile,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct SessionParams {
    /// Opaque identifier returned by `ffm_session_start` on this MCP server.
    pub(crate) session_id: SessionId,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct ActionParams {
    pub(crate) session_id: SessionId,
    /// Context name from `ffm_session_catalog`.
    pub(crate) context: String,
    /// Action name from `ffm_session_catalog`, interpreted by the current application state.
    pub(crate) action: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct InputParams {
    pub(crate) session_id: SessionId,
    /// Ordered FFM key chords, for example F4, Ctrl+S, Esc.
    #[serde(default)]
    pub(crate) keys: Vec<String>,
    /// Paste text into the active dialog, editor, command line, or running terminal.
    pub(crate) text: Option<String>,
    /// Optional mouse event; button defaults to left for press/release/drag.
    pub(crate) mouse: Option<MouseInput>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct WaitParams {
    pub(crate) session_id: SessionId,
    /// Maximum wait in milliseconds, 0..10000; default 1000.
    #[serde(default = "default_wait")]
    pub(crate) timeout_ms: u64,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct ResizeParams {
    pub(crate) session_id: SessionId,
    pub(crate) width: u16,
    pub(crate) height: u16,
}

fn default_width() -> u16 {
    120
}
fn default_height() -> u16 {
    40
}
fn default_wait() -> u64 {
    1000
}
