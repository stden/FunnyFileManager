//! Purpose: the F9 rows outside the panel, file and tool families, one typed command each.
//!
//! Most of these rows have no key and exist only in the menu; Save setup has
//! Shift+F9. A row whose function a key already names is not here: it holds
//! that key's `Action`, so the menu and the keyboard cannot drift apart.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuCommand {
    Performance,
    TokenUsage,
    SystemTempCleanup,
    LanguageServers,
    UpdatePackages,
    GitPullAll,
    Path(crate::path_env::Scope),
    Compare,
    /// Edit the user menu's file; F2 opens the menu itself.
    UserMenu,
    Associations,
    Shortcuts,
    Devices,
    McpInstall,
    /// Commands: the active folder's entries by size, with a verdict on each.
    SpaceAnalysis,
    /// The machines of the Things catalog, imported as sessions.
    ThingsCatalog,
    Settings(SettingsPage),
    /// `Shift+F9`.
    SaveSetup,
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum SettingsPage {
    #[default]
    System,
    Panel,
    Tree,
    Interface,
    Language,
    Plugins,
    PluginManager,
    Dialog,
    Menu,
    CommandLine,
    AutoComplete,
    Information,
    Masks,
    Confirmations,
    PanelModes,
    Descriptions,
    FolderDescriptions,
    Viewer,
    Editor,
    CodePages,
    Colors,
    Highlighting,
}

impl SettingsPage {
    /// The dialog's title, which is Far's row label without its hotkey marker.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::System => "System settings",
            Self::Panel => "Panel settings",
            Self::Tree => "Tree settings",
            Self::Interface => "Interface settings",
            Self::Language => "Languages",
            Self::Plugins => "Plugins configuration",
            Self::PluginManager => "Plugins manager settings",
            Self::Dialog => "Dialog settings",
            Self::Menu => "Menu settings",
            Self::CommandLine => "Command line settings",
            Self::AutoComplete => "AutoComplete settings",
            Self::Information => "InfoPanel settings",
            Self::Masks => "Groups of file masks",
            Self::Confirmations => "Confirmations",
            Self::PanelModes => "File panel modes",
            Self::Descriptions => "File descriptions",
            Self::FolderDescriptions => "Folder description files",
            Self::Viewer => "Viewer settings",
            Self::Editor => "Editor settings",
            Self::CodePages => "Code pages",
            Self::Colors => "Colors",
            Self::Highlighting => "Files highlighting and sort groups",
        }
    }
}

#[cfg(test)]
mod tests;
