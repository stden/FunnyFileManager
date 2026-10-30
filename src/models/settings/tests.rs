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

/// The values a save compares with follow the file: a load and a save both
/// record them, and a dialog edit in between does not.
#[test]
fn saved_values_follow_the_last_load_or_save() {
    let mut settings = Settings::default();
    settings.set(SettingsPage::Editor, "8").unwrap();
    assert_eq!(settings.saved_values().len(), 0);
    settings.mark_saved();
    settings.set(SettingsPage::Panel, "off").unwrap();
    assert_eq!(
        settings.saved_values(),
        &BTreeMap::from([(SettingsPage::Editor, "8".to_owned())])
    );
    settings.mark_saved();
    assert_eq!(settings.saved_values(), settings.values());
}

fn pages(entries: &[(SettingsPage, &str)]) -> PageValues {
    PageValues(
        entries
            .iter()
            .map(|(page, text)| (*page, (*text).to_owned()))
            .collect(),
    )
}

#[test]
fn page_values_are_saved_under_options_by_page_name() {
    let section = OptionsSection {
        options: pages(&[(SettingsPage::Editor, "8"), (SettingsPage::Panel, "off")]),
    };
    let yaml = serde_norway::to_string(&section).unwrap();
    assert_eq!(yaml, "options:\n  Panel: off\n  Editor: '8'\n");
    assert_eq!(
        serde_norway::from_str::<OptionsSection>(&yaml).unwrap(),
        section
    );
    let empty: OptionsSection = serde_norway::from_str("lsp: {}\n").unwrap();
    assert_eq!(empty, OptionsSection::default(), "no options, no pages");
}

/// The typed settings share the file, and a newer build may add pages.
#[test]
fn page_values_skip_every_key_that_names_no_page() {
    let yaml = "editor: {tab_size: 2}\nFuture: x\n1: x\nEditor: '8'\n";
    let values: PageValues = serde_norway::from_str(yaml).unwrap();
    assert_eq!(values, pages(&[(SettingsPage::Editor, "8")]));
    let json: PageValues = serde_json::from_str(r#"{"lsp":{},"Editor":"8"}"#).unwrap();
    assert_eq!(json, values);
}

/// A person writing YAML leaves numbers and switches unquoted. The text is
/// what YAML resolved, so a save that rewrites the file cannot change it:
/// `~` is empty before a save as well as after one writes it as `null`.
#[test]
fn page_values_read_any_yaml_scalar_as_the_text_it_resolves_to() {
    let yaml = "Editor: 8\nPanel: on\nMasks:\nPlugins: ~\nTree: +32\n\
                Descriptions: 0x10\nFolderDescriptions: 1e3\nMenu: true\n\
                CodePages: ' UTF-8 '\nConfirmations: null\n";
    let values: PageValues = serde_norway::from_str(yaml).unwrap();
    let expected = pages(&[
        (SettingsPage::Editor, "8"),
        (SettingsPage::Panel, "on"),
        (SettingsPage::Masks, ""),
        (SettingsPage::Plugins, ""),
        (SettingsPage::Tree, "32"),
        (SettingsPage::Descriptions, "16"),
        (SettingsPage::FolderDescriptions, "1000.0"),
        (SettingsPage::Menu, "true"),
        (SettingsPage::CodePages, "UTF-8"),
        (SettingsPage::Confirmations, ""),
    ]);
    assert_eq!(values, expected);
    let rewritten: serde_norway::Value = serde_norway::from_str(yaml).unwrap();
    let rewritten = serde_norway::to_string(&rewritten).unwrap();
    assert!(rewritten.contains("Plugins: null"), "{rewritten}");
    assert_eq!(
        serde_norway::from_str::<PageValues>(&rewritten).unwrap(),
        expected
    );
}

#[test]
fn page_values_refuse_a_duplicate_page_or_a_structured_value() {
    let duplicate = serde_norway::from_str::<PageValues>("Editor: '8'\nEditor: '9'\n");
    assert!(duplicate.unwrap_err().to_string().contains("Editor"));
    for yaml in ["Editor: [8]\n", "Editor: {a: 1}\n", "Masks: !x a\n"] {
        let error = serde_norway::from_str::<PageValues>(yaml).unwrap_err();
        assert!(
            error.to_string().contains("expected text, not a list"),
            "{error}"
        );
    }
    let sequence = serde_norway::from_str::<PageValues>("- Editor\n").unwrap_err();
    assert!(
        sequence.to_string().contains("a YAML mapping"),
        "{sequence}"
    );
}

/// `settings.json` kept the pages beside the typed keys, in its own order.
#[test]
fn legacy_pages_move_under_options_and_the_rest_stays() {
    let legacy: serde_norway::Mapping =
        serde_json::from_str(r#"{"Panel":"off","system":{"history_size":5},"Editor":"8"}"#)
            .unwrap();
    let document = nest_legacy_pages(legacy);
    let expected: serde_norway::Mapping =
        serde_norway::from_str("system: {history_size: 5}\noptions: {Panel: 'off', Editor: '8'}\n")
            .unwrap();
    assert_eq!(document, expected);
    let section: OptionsSection =
        serde_norway::from_value(serde_norway::Value::Mapping(document)).unwrap();
    assert_eq!(
        section.options,
        pages(&[(SettingsPage::Panel, "off"), (SettingsPage::Editor, "8")])
    );
    let typed_only: serde_norway::Mapping = serde_json::from_str(r#"{"lsp":{}}"#).unwrap();
    assert_eq!(nest_legacy_pages(typed_only.clone()), typed_only);
}
