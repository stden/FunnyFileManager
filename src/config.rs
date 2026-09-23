//! Purpose: Persistent user settings — one struct per page of Far's Options
//! menu, saved as JSON under the user's config directory.
//!
//! The page structs and their fields are fixed by the design spec (§5 D) so
//! that the dialog that edits a page and the code that consumes a setting
//! agree on the name. Every field here is read somewhere; a setting nothing
//! reads is not added. Loading never fails — a corrupt file is the defaults
//! plus a line in the error log — and saving is atomic, so a crash mid-write
//! leaves the previous file intact.

use std::{fs, io, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::models::PanelView;

mod lsp;
mod pages;
pub use lsp::{LspSettings, ServerSettings};

pub use pages::{
    AutoCompleteSettings, CodePageSettings, ColorSettings, CommandLineSettings,
    ConfirmationSettings, DescriptionSettings, DialogSettings, EditorSettings,
    FolderDescriptionSettings, HighlightingSettings, InfoPanelSettings, InterfaceSettings, Lang,
    LanguageSettings, MaskGroupSettings, MenuSettings, PanelModeSettings, PanelModesSettings,
    PanelSettings, PluginsConfigurationSettings, PluginsManagerSettings, SystemSettings, Theme,
    TreeSettings, ViewerSettings,
};

/// The settings file's name under the `ffm` config directory.
const FILE_NAME: &str = "settings.json";

/// Everything the Options menu edits, plus the folder shortcuts.
///
/// `#[serde(default)]` on every level: a file written by an older build, or
/// one the user trimmed by hand, loads with the missing pages at their
/// defaults instead of failing as a whole.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub lsp: LspSettings,
    pub system: SystemSettings,
    pub panel: PanelSettings,
    pub tree: TreeSettings,
    pub interface: InterfaceSettings,
    pub languages: LanguageSettings,
    pub plugins_configuration: PluginsConfigurationSettings,
    pub plugins_manager: PluginsManagerSettings,
    pub dialog: DialogSettings,
    pub menu: MenuSettings,
    pub command_line: CommandLineSettings,
    pub auto_complete: AutoCompleteSettings,
    pub info_panel: InfoPanelSettings,
    pub mask_groups: MaskGroupSettings,
    pub confirmations: ConfirmationSettings,
    pub panel_modes: PanelModesSettings,
    pub descriptions: DescriptionSettings,
    pub folder_descriptions: FolderDescriptionSettings,
    pub viewer: ViewerSettings,
    pub editor: EditorSettings,
    pub code_pages: CodePageSettings,
    pub colors: ColorSettings,
    pub highlighting: HighlightingSettings,
    /// `Ctrl+Shift+0` … `9`: the ten folder shortcuts, `None` where unset.
    pub shortcuts: [Option<PathBuf>; 10],
}

impl Settings {
    /// `$XDG_CONFIG_HOME/ffm/settings.json`, defaulting to `~/.config/…`;
    /// `%APPDATA%\ffm\settings.json` on Windows. `None` when the environment
    /// names no home at all — a relative `$XDG_CONFIG_HOME` is ignored, as the
    /// XDG spec says, so settings never land in the launch directory.
    #[must_use]
    pub fn path() -> Option<PathBuf> {
        dir().map(|dir| dir.join(FILE_NAME))
    }

    /// The saved settings, or the defaults when there is no file, it cannot be
    /// read, or it does not parse. Only the last two are worth a log line: a
    /// first run has no file, and that is not an error.
    #[must_use]
    pub fn load() -> Self {
        let Some(path) = Self::path() else {
            return Self::default();
        };
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Self::default(),
            Err(error) => {
                crate::diagnostics::log_error(&format!("settings: {}: {error}", path.display()));
                return Self::default();
            }
        };
        serde_json::from_str(&text).unwrap_or_else(|error| {
            crate::diagnostics::log_error(&format!("settings: {}: {error}", path.display()));
            Self::default()
        })
    }

    /// Write the settings where [`Self::load`] reads them, atomically: a
    /// sibling temp file replaces the old one in a single rename.
    pub fn save(&self) -> io::Result<()> {
        let path = Self::path()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no configuration directory"))?;
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        crate::history::write_owner_only(&path, &text)
    }

    /// The column layout and fullscreen flag configured for `view`.
    #[must_use]
    pub fn panel_mode(&self, view: PanelView) -> &PanelModeSettings {
        &self.panel_modes.modes[view.index()]
    }
}

/// The `ffm` directory every configuration file lives in — the settings, the
/// user menu and the file associations — so that they can be backed up as
/// one. `None` when the environment names no home at all.
#[must_use]
pub fn dir() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("ffm"))
}

#[cfg(windows)]
fn config_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(PathBuf::from)
}

#[cfg(not(windows))]
fn config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
}

#[cfg(test)]
mod tests;
