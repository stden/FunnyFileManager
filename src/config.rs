//! Purpose: Persistent user settings — one struct per page of Far's Options
//! menu, saved as YAML under the user's config directory.
//!
//! The page structs and their fields are fixed by the design spec (§5 D) so
//! that the dialog that edits a page and the code that consumes a setting
//! agree on the name. Every field here is read somewhere; a setting nothing
//! reads is not added. Loading never fails — a corrupt file is the defaults
//! plus a line in the error log — and saving is atomic, so a crash mid-write
//! leaves the previous file intact. How the file is shared with the Options
//! pages, migrated from JSON and protected from a save is in [`document`].

use std::{
    io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::models::PanelView;

pub(crate) mod document;
mod home;
mod lsp;
mod pages;
pub use home::{data_dir, dir, user_home};
pub use lsp::values::{
    quote, Arguments, Enablement, Extension, Extensions, LanguageId, ServerCommand, ValueError,
};
pub use lsp::{patch, InstallCommand, InstallRecipe, LspSettings, ServerSettings};

pub use pages::{
    AutoCompleteSettings, CodePageSettings, ColorSettings, CommandLineSettings,
    ConfirmationSettings, DescriptionSettings, DialogSettings, EditorSettings,
    FolderDescriptionSettings, HighlightingSettings, InfoPanelSettings, InterfaceSettings, Lang,
    LanguageSettings, LspTrust, MaskGroupSettings, MenuSettings, PanelModeSettings,
    PanelModesSettings, PanelSettings, PluginsConfigurationSettings, PluginsManagerSettings,
    SystemSettings, Theme, TreeSettings, ViewerSettings,
};

/// The top-level key of the language servers: serde's name for
/// [`Settings::lsp`], which the Language servers dialog edits in place.
pub(crate) const LSP_KEY: &str = "lsp";

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
    /// The settings as [`Self::load`] read them, which a save compares with
    /// the file so that it writes only what this session changed.
    #[serde(skip)]
    loaded: Loaded,
}

/// The settings a save compares against: those loaded, or `None` for the
/// defaults when nothing was loaded — no file, or one that failed to load.
///
/// Bookkeeping rather than a setting, so any two are equal: settings holding
/// the same values are the same settings however they were loaded.
#[derive(Debug, Clone, Default)]
struct Loaded(Option<Box<Settings>>);

impl PartialEq for Loaded {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for Loaded {}

impl Settings {
    /// `$XDG_CONFIG_HOME/ffm/settings.yaml`, defaulting to `~/.config/…`;
    /// `%APPDATA%\ffm\settings.yaml` on Windows. `None` when the environment
    /// names no home at all — a relative `$XDG_CONFIG_HOME` is ignored, as the
    /// XDG spec says, so settings never land in the launch directory.
    #[must_use]
    pub fn path() -> Option<PathBuf> {
        dir().map(|dir| dir.join(document::FILE_NAME))
    }

    /// The saved settings, or the defaults when there is no file, it cannot be
    /// read, or it does not parse. Only the last two are worth a log line: a
    /// first run has no file, and that is not an error.
    #[must_use]
    pub fn load() -> Self {
        let Some(path) = Self::path() else {
            return Self::default();
        };
        Self::load_from(&path).unwrap_or_else(|error| {
            crate::diagnostics::log_error(&format!("settings: {}: {error}", path.display()));
            Self::default()
        })
    }

    /// The settings stored at `path`, or in the legacy JSON file beside it;
    /// the defaults when neither file exists.
    pub(crate) fn load_from(path: &Path) -> io::Result<Self> {
        let Some(stored) = document::read(path)? else {
            return Ok(Self::default());
        };
        let mut settings: Self = stored.parse()?;
        settings.loaded = Loaded(Some(Box::new(settings.clone())));
        Ok(settings)
    }

    /// Write the settings where [`Self::load`] reads them, atomically: a
    /// sibling temp file replaces the old one in a single rename. Refused,
    /// with the file untouched, when the stored file does not parse or
    /// changed a page this session changed too. The comparison is always with
    /// what was loaded, so after a save of a change, a hand edit of the same
    /// value refuses the next save rather than being overwritten.
    pub fn save(&self) -> io::Result<()> {
        let path = Self::path()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no configuration directory"))?;
        self.save_to(&path)
    }

    /// Write the pages this session changed into the settings file at
    /// `path`, keeping the Options pages, hand edits and every other key.
    fn save_to(&self, path: &Path) -> io::Result<()> {
        let defaults = Self::default();
        let loaded = self.loaded.0.as_deref().unwrap_or(&defaults);
        document::update(path, loaded, self)
    }

    /// Take `lsp`, which a Language servers save just wrote, as the servers
    /// this session runs *and* as the ones it loaded, so the exit save finds
    /// `lsp` unchanged and leaves it to the file: writing it would pin every
    /// catalog field, and a later hand edit of the same field would read as a
    /// conflict.
    pub(crate) fn adopt_saved_lsp(&mut self, lsp: LspSettings) {
        let mut loaded = self.loaded.0.take().unwrap_or_default();
        loaded.lsp.clone_from(&lsp);
        self.loaded = Loaded(Some(loaded));
        self.lsp = lsp;
    }

    /// The column layout and fullscreen flag configured for `view`.
    #[must_use]
    pub fn panel_mode(&self, view: PanelView) -> &PanelModeSettings {
        let modes = &self.panel_modes.modes;
        let [first, ..] = modes;
        // `modes` has a slot for every position `index` returns; the fallback
        // mirrors `index`'s own, the first mode.
        modes.get(view.index()).unwrap_or(first)
    }
}

#[cfg(test)]
mod tests;
