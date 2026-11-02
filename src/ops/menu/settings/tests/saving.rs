//! Purpose: Save setup keeps typed and hand-edited pages, and needs a file.

use super::*;

/// Save setup used to write only the page map, erasing the language servers
/// and every typed page; the first save also migrates the legacy JSON.
#[test]
fn save_setup_keeps_the_typed_settings_and_leaves_the_legacy_json_alone() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let path = temp.path().join("settings.yaml");
    let legacy = temp.path().join("settings.json");
    let json = r#"{"lsp":{"python":{"enabled":false}},"system":{"history_size":5},"Panel":"off"}"#;
    fs::write(&legacy, json)?;
    let mut app = App::new()?;
    load(&mut app, &path)?;
    app.settings.set(SettingsPage::Editor, "8").unwrap();
    save(&mut app)?;
    assert_eq!(fs::read_to_string(&legacy)?, json);
    let stored = document::read(&path)?.expect("settings.yaml");
    let typed: crate::config::Settings = stored.parse()?;
    assert_eq!(
        typed.lsp.0["python"].enabled,
        crate::config::Enablement::Disabled
    );
    assert_eq!(typed.system.history_size, 5);
    let OptionsSection {
        options: PageValues(pages),
    } = stored.parse()?;
    assert_eq!(pages[&SettingsPage::Panel], "off");
    assert_eq!(pages[&SettingsPage::Editor], "8");
    Ok(())
}

#[test]
fn save_without_a_file_is_an_error() {
    let mut app = App::new().expect("app");
    app.settings.persistence = Persistence::SessionOnly;
    assert!(save(&mut app).is_err());
}

/// Save setup writes the pages changed in this session and keeps a page the
/// user changed in the file meanwhile; after a save, the file is the new
/// reference, so a later hand edit survives the next save too.
#[test]
fn save_setup_keeps_a_page_edited_by_hand_while_ffm_runs() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let path = temp.path().join("settings.yaml");
    fs::write(&path, "options: {Editor: '8'}\n")?;
    let mut app = app_in(&temp)?;
    load(&mut app, &path)?;
    fs::write(&path, "options: {Editor: '2'}\n")?;
    app.settings.set(SettingsPage::Panel, "off").unwrap();
    save(&mut app)?;
    let expected = OptionsSection {
        options: PageValues(BTreeMap::from([
            (SettingsPage::Panel, "off".to_owned()),
            (SettingsPage::Editor, "2".to_owned()),
        ])),
    };
    let stored = document::read(&path)?.expect("saved");
    assert_eq!(stored.parse::<OptionsSection>()?, expected);
    fs::write(&path, "options: {Editor: '3', Panel: 'off'}\n")?;
    save(&mut app)?;
    assert_eq!(
        fs::read_to_string(&path)?,
        "options: {Editor: '3', Panel: 'off'}\n",
        "nothing changed since the last save"
    );
    Ok(())
}

#[test]
fn a_page_changed_by_hand_and_in_the_dialog_is_not_saved() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let path = temp.path().join("settings.yaml");
    fs::write(&path, "options: {Editor: '8'}\n")?;
    let mut app = app_in(&temp)?;
    load(&mut app, &path)?;
    fs::write(&path, "options: {Editor: '2'}\n")?;
    app.settings.set(SettingsPage::Editor, "6").unwrap();
    let error = save(&mut app).expect_err("both changed the Editor page");
    assert_eq!(
        error.to_string(),
        "Not saved: settings.yaml changed on disk: options.Editor was edited there and in \
         this session"
    );
    assert_eq!(fs::read_to_string(&path)?, "options: {Editor: '2'}\n");
    Ok(())
}
