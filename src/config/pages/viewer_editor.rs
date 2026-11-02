//! Purpose: The pages the built-in viewer and editor read: viewer, editor,
//! and code pages.

use serde::{Deserialize, Serialize};

/// Viewer settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewerSettings {
    pub tab_size: u32,
    /// Off, as in Far: long lines pan with `Left`/`Right`.
    pub wrap: bool,
    pub default_encoding: String,
}

impl Default for ViewerSettings {
    fn default() -> Self {
        Self {
            tab_size: 8,
            wrap: false,
            default_encoding: "UTF-8".to_owned(),
        }
    }
}

/// Workspace approval policy for editor language servers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LspTrust {
    /// Ask before launching or installing servers for a new workspace.
    #[default]
    Prompt,
    /// Approve every workspace for the configured, enabled servers.
    Automatic,
}

/// Editor settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorSettings {
    pub tab_size: u32,
    pub expand_tabs: bool,
    pub auto_indent: bool,
    /// Reopen a file at the line it was closed on.
    pub save_position: bool,
    pub lsp_trust: LspTrust,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            tab_size: 8,
            expand_tabs: false,
            auto_indent: false,
            save_position: true,
            lsp_trust: LspTrust::Prompt,
        }
    }
}

/// Code pages: the encodings the viewer cycles through.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CodePageSettings {
    pub favorites: Vec<String>,
}

impl Default for CodePageSettings {
    fn default() -> Self {
        Self {
            favorites: vec!["UTF-8".to_owned(), "ANSI".to_owned()],
        }
    }
}
