//! Purpose: The page structs of `Settings`, one per row of Far's Options
//! menu, with Far's defaults.
//!
//! Field names are the contract between the dialog page that edits them
//! (`src/config/pages/<page>.rs`, owned by each page's agent) and the code
//! that reads them; the design spec's table (§5 D) lists both sides.

use serde::{Deserialize, Serialize};

mod appearance;
mod panels;
mod shell;
mod viewer_editor;

pub use appearance::{ColorSettings, HighlightingSettings, InterfaceSettings, Theme};
pub use panels::{
    DescriptionSettings, FolderDescriptionSettings, InfoPanelSettings, MaskGroupSettings,
    PanelModeSettings, PanelModesSettings, PanelSettings, TreeSettings,
};
pub use shell::{
    AutoCompleteSettings, CommandLineSettings, ConfirmationSettings, DialogSettings, MenuSettings,
};
pub use viewer_editor::{CodePageSettings, EditorSettings, ViewerSettings};

/// System settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SystemSettings {
    /// Write the settings back at exit without asking (Far: off, hence
    /// `Shift+F9`).
    pub auto_save_setup: bool,
    /// `F8` overwrites before unlinking.
    pub wipe_on_delete: bool,
    /// Lines the command history keeps.
    pub history_size: u32,
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            auto_save_setup: false,
            wipe_on_delete: false,
            history_size: crate::history::HISTORY_CAP as u32,
        }
    }
}

/// The two languages the interface and the help can be shown in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Lang {
    #[default]
    En,
    Ru,
}

/// Languages.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LanguageSettings {
    pub main: Lang,
    pub help: Lang,
}

/// Plugins configuration: which MCP servers from `.mcp.json` are switched off.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginsConfigurationSettings {
    pub disabled: Vec<String>,
}

/// Plugins manager settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginsManagerSettings {
    /// Read `.mcp.json` from the panel's folder for `F11`.
    pub scan_project_mcp: bool,
    pub show_status_on_start: bool,
}

impl Default for PluginsManagerSettings {
    fn default() -> Self {
        Self {
            scan_project_mcp: true,
            show_status_on_start: false,
        }
    }
}
