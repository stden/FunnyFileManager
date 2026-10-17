//! Purpose: Loading settings: the default file, failures, languages, saved setups and the legacy JSON.

use super::*;

#[test]
fn load_default_uses_the_settings_file_under_the_config_home() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let _home = EnvVarGuard::set(CONFIG_HOME, temp.path());
    let path = temp.path().join("ffm").join("settings.yaml");
    write_file(&path, "options:\n  Editor: '8'\n")?;
    let mut app = app_in(&temp)?;
    load_default(&mut app);
    assert_eq!(app.settings.persistence.file(), Some(path.as_path()));
    assert_eq!(app.settings.number(SettingsPage::Editor), 8);
    assert!(!matches!(app.status, Status::Error(_)), "{}", app.status);
    Ok(())
}

/// The failed file stays the save target, so Save setup can say why it will
/// not overwrite the file instead of claiming there is none.
#[test]
fn a_settings_file_that_fails_to_load_is_reported_and_never_overwritten() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let path = temp.path().join("ffm").join("settings.yaml");
    write_file(&path, "Editor: [\n")?;
    let mut app = app_in(&temp)?;
    restore(&mut app, path.clone());
    let status = app.status.to_string();
    assert!(
        status.contains("Settings: settings.yaml does not parse"),
        "{status}"
    );
    assert_eq!(app.settings.persistence.file(), Some(path.as_path()));
    let error = save(&mut app).expect_err("the broken file is not replaced");
    let message = error.to_string();
    assert!(
        message.starts_with("Not saved: settings.yaml does not parse"),
        "{message}"
    );
    assert_eq!(fs::read_to_string(&path)?, "Editor: [\n");
    Ok(())
}

#[test]
fn unavailable_language_rolls_back_loaded_panel_settings() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    fs::write(temp.path().join(".hidden"), "keep visible")?;
    let panel = crate::models::Panel::new(temp.path().to_owned())?;
    let mut app = App::from_panels(panel.clone(), panel);
    let path = temp.path().join("settings.yaml");
    let values = BTreeMap::from([
        (SettingsPage::Panel, "off".to_owned()),
        (
            SettingsPage::Language,
            temp.path().join("missing.json").display().to_string(),
        ),
    ]);
    let section = OptionsSection {
        options: PageValues(values),
    };
    fs::write(
        &path,
        serde_norway::to_string(&section).map_err(io::Error::other)?,
    )?;
    assert!(load(&mut app, &path).is_err());
    assert!(app.settings.enabled(SettingsPage::Panel));
    assert!(!app.left_panel.options.hide_hidden);
    assert!(app
        .left_panel
        .entries
        .iter()
        .any(|entry| entry.name == ".hidden"));
    Ok(())
}

#[test]
fn saved_setup_restores_validated_settings() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let path = temp.path().join("config/settings.yaml");
    let mut app = App::new()?;
    load(&mut app, &path)?;
    app.settings.set(SettingsPage::Editor, "8").unwrap();
    save(&mut app)?;
    assert!(fs::read_to_string(&path)?.contains("\noptions:\n  Editor: '8'\n"));
    let mut restored = App::new()?;
    load(&mut restored, &path)?;
    assert_eq!(restored.settings.number(SettingsPage::Editor), 8);
    assert!(restored.settings.enabled(SettingsPage::Panel));
    Ok(())
}

#[test]
fn invalid_setup_does_not_replace_the_current_settings() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let path = temp.path().join("settings.yaml");
    fs::write(&path, "options: {Editor: '0'}\n")?;
    let mut app = App::new()?;
    let error = load(&mut app, &path).expect_err("0 is out of range");
    assert!(
        error.to_string().starts_with("Editor settings: "),
        "{error}"
    );
    assert_eq!(app.settings.number(SettingsPage::Editor), 4);
    Ok(())
}

#[test]
fn the_options_pages_read_the_legacy_json_until_there_is_yaml() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let path = temp.path().join("settings.yaml");
    fs::write(temp.path().join("settings.json"), r#"{"Editor":"8"}"#)?;
    let mut app = App::new()?;
    load(&mut app, &path)?;
    assert_eq!(app.settings.number(SettingsPage::Editor), 8);
    fs::write(&path, "options: {Editor: 2}\n")?;
    load(&mut app, &path)?;
    assert_eq!(app.settings.number(SettingsPage::Editor), 2, "YAML wins");
    Ok(())
}

/// A legacy file in the way is named at startup and when Save setup refuses.
#[test]
fn a_legacy_settings_json_that_cannot_be_read_is_named() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let path = temp.path().join("config").join("settings.yaml");
    fs::create_dir_all(temp.path().join("config").join("settings.json"))?;
    let mut app = app_in(&temp)?;
    restore(&mut app, path.clone());
    assert_eq!(
        app.status.to_string(),
        "Error: Settings: settings.json is not a regular file"
    );
    let error = save(&mut app).expect_err("refused");
    assert_eq!(
        error.to_string(),
        "Not saved: settings.json is not a regular file"
    );
    assert!(!path.exists());
    Ok(())
}
