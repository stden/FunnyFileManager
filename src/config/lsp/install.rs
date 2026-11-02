//! Purpose: Typed user-level installation recipes; commands are argument arrays, never shell text.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InstallRecipe {
    Python {
        package: String,
        module: String,
    },
    Npm {
        packages: Vec<String>,
        package: String,
        binary: String,
    },
    Jdtls {
        url: String,
        sha256: String,
    },
    Command {
        steps: Vec<InstallCommand>,
        executable: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallCommand {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}
