use super::SettingsPage;

#[test]
fn every_settings_page_has_a_plain_title() {
    let pages = [
        SettingsPage::System,
        SettingsPage::Panel,
        SettingsPage::Tree,
        SettingsPage::Interface,
        SettingsPage::Language,
        SettingsPage::Plugins,
        SettingsPage::PluginManager,
        SettingsPage::Dialog,
        SettingsPage::Menu,
        SettingsPage::CommandLine,
        SettingsPage::AutoComplete,
        SettingsPage::Information,
        SettingsPage::Masks,
        SettingsPage::Confirmations,
        SettingsPage::PanelModes,
        SettingsPage::Descriptions,
        SettingsPage::FolderDescriptions,
        SettingsPage::Viewer,
        SettingsPage::Editor,
        SettingsPage::CodePages,
        SettingsPage::Colors,
        SettingsPage::Highlighting,
    ];
    for page in pages {
        let title = page.title();
        assert_ne!(title.len(), 0, "{page:?}");
        assert!(!title.contains('&'), "{title}");
    }
}
