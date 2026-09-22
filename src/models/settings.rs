//! Validated settings values and the production configuration location.

use crate::menu::SettingsPage;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct Settings {
    values: BTreeMap<SettingsPage, String>,
    pub translations: BTreeMap<String, String>,
    pub persistence: crate::history::Persistence,
}

impl Settings {
    #[must_use]
    pub fn translate<'a>(&'a self, label: &'a str) -> &'a str {
        self.translations.get(label).map_or(label, String::as_str)
    }
    #[must_use]
    pub fn with_file(path: std::path::PathBuf) -> Self {
        Self {
            persistence: crate::history::Persistence::File(path),
            ..Self::default()
        }
    }
    #[must_use]
    pub fn get(&self, page: SettingsPage) -> &str {
        self.values
            .get(&page)
            .map_or_else(|| specification(page).1, String::as_str)
    }

    pub fn set(&mut self, page: SettingsPage, text: &str) -> Result<(), String> {
        let text = text.trim();
        validate(page, text)?;
        self.values.insert(page, text.to_owned());
        Ok(())
    }

    #[must_use]
    pub fn enabled(&self, page: SettingsPage) -> bool {
        self.get(page) == "on"
    }

    #[must_use]
    pub fn number(&self, page: SettingsPage) -> usize {
        self.get(page).parse().unwrap_or(1)
    }

    #[must_use]
    pub fn values(&self) -> &BTreeMap<SettingsPage, String> {
        &self.values
    }
}

pub fn specification(page: SettingsPage) -> (&'static str, &'static str, &'static str) {
    use SettingsPage as P;
    match page {
        P::System => ("System settings", "100000", "Maximum entries visited by file/folder searches (1–1000000)"),
        P::Panel => ("Panel settings", "on", "Show hidden files: on/off"),
        P::Tree => ("Tree settings", "32", "Maximum search/tree depth (1–256)"),
        P::Interface => ("Interface settings", "on", "Show the menu clock: on/off"),
        P::Language => ("Languages", "English", "Menu language: English or a path to a JSON label translation dictionary"),
        P::Plugins => ("Plugins configuration", "", "Plugin command to offer from the user menu"),
        P::PluginManager => ("Plugins manager settings", "10", "Shell and plugin command timeout in seconds (1–3600)"),
        P::Dialog => ("Dialog settings", "on", "Allow Escape to dismiss menu prompts: on/off"),
        P::Menu => ("Menu settings", "on", "Wrap menu navigation at the first/last entry: on/off"),
        P::CommandLine => ("Command line settings", "500", "Number of recent commands shown by History (1–500)"),
        P::AutoComplete => ("AutoComplete settings", "on", "Command-line completion and suggestions: on/off"),
        P::Information => ("InfoPanel settings", "on", "Include folder description files in quick view: on/off"),
        P::Masks => ("Groups of file masks", "", "Default group-selection mask (semicolon-separated patterns); empty uses the selected extension"),
        P::Confirmations => ("Confirmations", "on", "Confirm file deletion: on/off"),
        P::PanelModes => ("File panel modes", "Full", "Default view: Brief, Medium, Full, Wide, Detailed, Descriptions, Long descriptions, File owners, File links, Alternative full"),
        P::Descriptions => ("File descriptions", "descript.ion", "Description sidecar filename"),
        P::FolderDescriptions => ("Folder description files", "README.md;README.txt", "Folder description filenames separated by semicolons"),
        P::Viewer => ("Viewer settings", "4", "Tab display width (1–16)"),
        P::Editor => ("Editor settings", "4", "Spaces inserted by Tab (1–16)"),
        P::CodePages => ("Code pages", "UTF-8", "Default encoding for new files: UTF-8, UTF-16LE, UTF-16BE, Windows-1251, Windows-1252"),
        P::Colors => ("Colors", "blue", "Panel background: blue, black, gray"),
        P::Highlighting => ("Files highlighting and sort groups", "on", "File-type colors in panels: on/off"),
    }
}

const CODE_PAGES: [&str; 5] = [
    "UTF-8",
    "UTF-16LE",
    "UTF-16BE",
    "Windows-1251",
    "Windows-1252",
];

fn validate(page: SettingsPage, text: &str) -> Result<(), String> {
    use SettingsPage as P;
    match page {
        P::Panel
        | P::Interface
        | P::Dialog
        | P::Menu
        | P::AutoComplete
        | P::Information
        | P::Confirmations
        | P::Highlighting => one_of(text, &["on", "off"]),
        P::System => number(text, 1_000_000),
        P::Tree => number(text, 256),
        P::PluginManager => number(text, 3600),
        P::CommandLine => number(text, 500),
        P::Viewer | P::Editor => number(text, 16),
        P::Colors => one_of(text, &["blue", "black", "gray"]),
        P::PanelModes => one_of(text, &super::PanelView::ALL.map(super::PanelView::label)),
        P::CodePages => one_of(text, &CODE_PAGES),
        P::Descriptions if text.is_empty() || text.contains(['/', '\\']) => {
            Err("Enter one filename".to_owned())
        }
        _ => Ok(()),
    }
}

fn one_of(text: &str, values: &[&str]) -> Result<(), String> {
    if values.contains(&text) {
        Ok(())
    } else {
        Err(format!("Choose {}", values.join(", ")))
    }
}

fn number(text: &str, maximum: usize) -> Result<(), String> {
    if text
        .parse::<usize>()
        .is_ok_and(|value| (1..=maximum).contains(&value))
    {
        Ok(())
    } else {
        Err(format!("Enter a number from 1 to {maximum}"))
    }
}

#[cfg(test)]
mod tests;
