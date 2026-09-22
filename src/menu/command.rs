//! Typed commands for menu operations that also have keyboard entry points.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuCommand {
    Performance,
    UpdatePackages,
    Path(crate::path_env::Scope),
    Tree,
    QuickView,
    Sort,
    LongNames,
    TogglePanel(crate::models::PanelSide),
    Link,
    Wipe,
    Archive,
    Extract,
    ArchiveCommands,
    Attributes,
    ApplyCommand,
    Describe,
    FindFile,
    History,
    VideoMode,
    FindFolder,
    ViewHistory,
    FolderHistory,
    Compare,
    UserMenu,
    Associations,
    Shortcuts,
    Filter,
    Screens,
    Tasks,
    Devices,
    Settings(SettingsPage),
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
