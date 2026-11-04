//! Steps for F9 Options menu settings page interactions.

use super::BehaviorWorld;
use crate::menu::SettingsPage;
use cucumber::when;

#[when(expr = "I open the settings page {string}")]
fn open_settings_page(world: &mut BehaviorWorld, page: String) {
    let settings_page = all_settings_pages()
        .into_iter()
        .find(|p| p.title() == page)
        .unwrap_or_else(|| panic!("unknown settings page: {page}"));
    let case = world.case();
    crate::ops::menu::settings::open(&mut case.app, settings_page);
    case.paint();
}

fn all_settings_pages() -> Vec<SettingsPage> {
    use SettingsPage::{
        AutoComplete, CodePages, Colors, CommandLine, Confirmations, Descriptions, Dialog,
        Editor, FolderDescriptions, Highlighting, Information, Interface, Language, Masks,
        Menu, PanelModes, Panel, Plugins, PluginManager, System, Tree, Viewer,
    };
    vec![
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
    ]
}
