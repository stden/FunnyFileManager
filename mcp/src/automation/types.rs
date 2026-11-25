//! Purpose: Typed automation requests and opaque, server-owned session identifiers.

use super::Error;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(
    Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize, schemars::JsonSchema,
)]
#[serde(transparent)]
pub struct SessionId(pub(super) String);

#[derive(Clone, Copy)]
pub struct Size {
    pub(super) width: u16,
    pub(super) height: u16,
}

impl Size {
    pub(crate) fn new(width: u16, height: u16) -> Result<Self, Error> {
        if !(20..=400).contains(&width) || !(8..=200).contains(&height) {
            return Err(Error::Invalid("width must be 20..400 and height 8..200"));
        }
        Ok(Self { width, height })
    }
}

#[derive(Deserialize, Serialize, schemars::JsonSchema)]
pub struct MouseInput {
    pub(crate) kind: String,
    pub(crate) button: Option<String>,
    pub(crate) column: u16,
    pub(crate) row: u16,
    #[serde(default)]
    pub(crate) modifiers: Vec<String>,
}

#[derive(Serialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum Request {
    Catalog,
    State,
    Action {
        context: String,
        action: String,
    },
    Input {
        keys: Vec<String>,
        text: Option<String>,
        mouse: Option<MouseInput>,
    },
    Wait {
        timeout_ms: u64,
    },
    Resize {
        width: u16,
        height: u16,
    },
    Cancel,
    Close,
}

#[derive(Serialize)]
pub struct Started {
    pub(crate) session_id: SessionId,
    pub(crate) protocol_version: u64,
    pub(crate) state: Value,
}

#[derive(Clone, Copy, Default, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    #[default]
    System,
    Isolated,
}
