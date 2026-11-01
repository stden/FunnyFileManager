//! Purpose: Versioned input messages keep session operations explicit and size-bounded.
use serde::Deserialize;
pub(super) mod input;

pub(super) const VERSION: u64 = 1;
pub(super) const MAX_FRAME: usize = 4 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Request {
    Catalog {},
    State {},
    Action {
        context: String,
        action: String,
    },
    Input {
        #[serde(default)]
        keys: Vec<String>,
        text: Option<String>,
        mouse: Option<Mouse>,
    },
    Wait {
        #[serde(default = "wait_timeout")]
        timeout_ms: u64,
    },
    Resize {
        width: u16,
        height: u16,
    },
    Cancel {},
    Close {},
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Mouse {
    pub kind: String,
    pub button: Option<String>,
    pub column: u16,
    pub row: u16,
    #[serde(default)]
    pub modifiers: Vec<String>,
}

const fn wait_timeout() -> u64 {
    1000
}
