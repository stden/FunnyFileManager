use super::*;

#[test]
fn settings_validate_before_replacing_the_previous_value() {
    let mut settings = Settings::default();
    assert_eq!(settings.get(SettingsPage::Editor), "4");
    settings.set(SettingsPage::Editor, "8").unwrap();
    assert!(settings.set(SettingsPage::Editor, "0").is_err());
    assert_eq!(settings.number(SettingsPage::Editor), 8);
    assert!(settings.set(SettingsPage::Colors, "invalid").is_err());
    settings.set(SettingsPage::Interface, "off").unwrap();
    assert!(!settings.enabled(SettingsPage::Interface));
}
